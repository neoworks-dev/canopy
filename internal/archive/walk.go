package archive

import (
	"archive/tar"
	"archive/zip"
	"compress/bzip2"
	"compress/gzip"
	"io"
	"io/fs"
	"os"
	"path"
	"strings"
	"time"
)

// Member is one entry inside an archive. The name is the path it holds inside,
// always slash separated and never leading with one.
type Member struct {
	Name    string `json:"name"`
	Size    int64  `json:"size"`
	IsDir   bool   `json:"isDir"`
	ModTime string `json:"modTime"`
	Mode    string `json:"mode"`
}

// Walk hands each member of an archive to visit, in the order the archive holds
// them. The reader is only good for the duration of the call: a tar is a stream,
// so nothing can be read back once the walk has moved on.
//
// A visit that returns an error stops the walk and that error is what Walk
// reports, which is how a caller cancels.
func Walk(archivePath string, visit func(member Member, contents io.Reader) error) error {
	format := FormatOf(archivePath)
	if format == FormatZip {
		return walkZip(archivePath, visit)
	}
	if format == FormatUnknown {
		return ErrUnsupportedFormat
	}
	return walkTar(archivePath, format, visit)
}

// List reports what an archive holds without reading any of it.
func List(archivePath string) ([]Member, error) {
	members := []Member{}
	err := Walk(archivePath, func(member Member, _ io.Reader) error {
		members = append(members, member)
		return nil
	})
	if err != nil {
		return nil, err
	}
	return members, nil
}

func walkZip(archivePath string, visit func(member Member, contents io.Reader) error) error {
	reader, err := zip.OpenReader(archivePath)
	if err != nil {
		return err
	}
	defer reader.Close()
	for _, file := range reader.File {
		err = visitZipEntry(file, visit)
		if err != nil {
			return err
		}
	}
	return nil
}

func visitZipEntry(file *zip.File, visit func(member Member, contents io.Reader) error) error {
	member := memberOf(file.Name, file.FileInfo())
	if member.IsDir {
		return visit(member, strings.NewReader(""))
	}
	contents, err := file.Open()
	if err != nil {
		return err
	}
	defer contents.Close()
	return visit(member, contents)
}

func walkTar(
	archivePath string,
	format Format,
	visit func(member Member, contents io.Reader) error,
) error {
	file, err := os.Open(archivePath)
	if err != nil {
		return err
	}
	defer file.Close()
	stream, closeStream, err := decompressed(file, format)
	if err != nil {
		return err
	}
	defer closeStream()
	return visitTarEntries(tar.NewReader(stream), visit)
}

func visitTarEntries(reader *tar.Reader, visit func(member Member, contents io.Reader) error) error {
	for {
		header, err := reader.Next()
		if err == io.EOF {
			return nil
		}
		if err != nil {
			return err
		}
		err = visitTarEntry(header, reader, visit)
		if err != nil {
			return err
		}
	}
}

// Only files and directories are handed on. A tar can also hold links and
// devices, which an extraction has no business recreating.
func visitTarEntry(
	header *tar.Header,
	reader *tar.Reader,
	visit func(member Member, contents io.Reader) error,
) error {
	if header.Typeflag != tar.TypeReg && header.Typeflag != tar.TypeDir {
		return nil
	}
	return visit(memberOf(header.Name, header.FileInfo()), reader)
}

// decompressed unwraps the compression a tar is packed in, reporting how to
// close whatever it had to open.
func decompressed(file *os.File, format Format) (io.Reader, func(), error) {
	if format == FormatTarGzip {
		return gzipStream(file)
	}
	if format == FormatTarBzip2 {
		return bzip2.NewReader(file), func() {}, nil
	}
	return file, func() {}, nil
}

func gzipStream(file *os.File) (io.Reader, func(), error) {
	reader, err := gzip.NewReader(file)
	if err != nil {
		return nil, func() {}, err
	}
	return reader, func() { reader.Close() }, nil
}

func memberOf(name string, info fs.FileInfo) Member {
	cleaned := path.Clean(strings.TrimPrefix(name, "/"))
	return Member{
		Name:    cleaned,
		Size:    info.Size(),
		IsDir:   info.IsDir() || strings.HasSuffix(name, "/"),
		ModTime: info.ModTime().Format(time.RFC3339),
		Mode:    info.Mode().String(),
	}
}
