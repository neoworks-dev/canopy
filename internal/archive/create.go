package archive

import (
	"archive/tar"
	"archive/zip"
	"compress/gzip"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"strings"
)

// packer is the writing half of a format. The two the explorer packs into differ
// in how an entry is announced and in nothing else.
type packer interface {
	add(name string, path string, info fs.FileInfo) error
	Close() error
}

// Create writes an archive holding the sources, keeping each source's own name
// as the top of what it holds. report is called with every entry written, and an
// error from it stops the run, which is how a caller cancels.
//
// An entry that cannot be read is left out rather than failing the archive: one
// unreadable file is a worse reason to have no archive at all.
func Create(archivePath string, sources []string, report func(name string, written int64) error) error {
	pack, err := packerFor(archivePath)
	if err != nil {
		return err
	}
	err = addAll(pack, sources, report)
	if err != nil {
		pack.Close()
		return err
	}
	return pack.Close()
}

func addAll(pack packer, sources []string, report func(name string, written int64) error) error {
	for _, source := range sources {
		err := addTree(pack, source, report)
		if err != nil {
			return err
		}
	}
	return nil
}

func addTree(pack packer, source string, report func(name string, written int64) error) error {
	base := filepath.Dir(source)
	return filepath.WalkDir(source, func(path string, entry fs.DirEntry, walkErr error) error {
		if walkErr != nil {
			return nil
		}
		name, info, ok := entryToPack(base, path, entry)
		if !ok {
			return nil
		}
		return packOne(pack, name, path, info, report)
	})
}

func packOne(
	pack packer,
	name string,
	path string,
	info fs.FileInfo,
	report func(name string, written int64) error,
) error {
	err := pack.add(name, path, info)
	if err != nil {
		return err
	}
	return report(name, info.Size())
}

// entryToPack reports the name an entry takes inside the archive. Only files and
// directories go in: a link or a device is not something to unpack again.
func entryToPack(base, path string, entry fs.DirEntry) (string, fs.FileInfo, bool) {
	if !entry.IsDir() && !entry.Type().IsRegular() {
		return "", nil, false
	}
	info, err := entry.Info()
	if err != nil {
		return "", nil, false
	}
	relative, err := filepath.Rel(base, path)
	if err != nil {
		return "", nil, false
	}
	return filepath.ToSlash(relative), info, true
}

func packerFor(archivePath string) (packer, error) {
	format := FormatOf(archivePath)
	file, err := os.Create(archivePath)
	if err != nil {
		return nil, err
	}
	pack, err := packerOfFormat(file, format)
	if err != nil {
		file.Close()
		os.Remove(archivePath)
		return nil, err
	}
	return pack, nil
}

func packerOfFormat(file *os.File, format Format) (packer, error) {
	if format == FormatZip {
		return &zipPacker{file: file, writer: zip.NewWriter(file)}, nil
	}
	if format == FormatTar {
		return &tarPacker{file: file, writer: tar.NewWriter(file)}, nil
	}
	if format == FormatTarGzip {
		compressor := gzip.NewWriter(file)
		return &tarPacker{file: file, compressor: compressor, writer: tar.NewWriter(compressor)}, nil
	}
	if format == FormatTarBzip2 {
		return nil, ErrCannotWriteFormat
	}
	return nil, ErrUnsupportedFormat
}

type zipPacker struct {
	file   *os.File
	writer *zip.Writer
}

func (pack *zipPacker) add(name string, path string, info fs.FileInfo) error {
	header, err := zip.FileInfoHeader(info)
	if err != nil {
		return err
	}
	header.Name = zipEntryName(name, info)
	header.Method = zip.Deflate
	entry, err := pack.writer.CreateHeader(header)
	if err != nil || info.IsDir() {
		return err
	}
	return copyInto(entry, path)
}

func (pack *zipPacker) Close() error {
	err := pack.writer.Close()
	if err != nil {
		pack.file.Close()
		return err
	}
	return pack.file.Close()
}

// A zip marks a directory by the trailing slash on its name and by nothing else.
func zipEntryName(name string, info fs.FileInfo) string {
	if info.IsDir() && !strings.HasSuffix(name, "/") {
		return name + "/"
	}
	return name
}

type tarPacker struct {
	file       *os.File
	compressor *gzip.Writer
	writer     *tar.Writer
}

func (pack *tarPacker) add(name string, path string, info fs.FileInfo) error {
	header, err := tar.FileInfoHeader(info, "")
	if err != nil {
		return err
	}
	header.Name = name
	err = pack.writer.WriteHeader(header)
	if err != nil || info.IsDir() {
		return err
	}
	return copyInto(pack.writer, path)
}

func (pack *tarPacker) Close() error {
	err := pack.writer.Close()
	if err != nil {
		pack.closeRest()
		return err
	}
	return pack.closeRest()
}

func (pack *tarPacker) closeRest() error {
	if pack.compressor != nil {
		err := pack.compressor.Close()
		if err != nil {
			pack.file.Close()
			return err
		}
	}
	return pack.file.Close()
}

func copyInto(entry io.Writer, path string) error {
	file, err := os.Open(path)
	if err != nil {
		return err
	}
	defer file.Close()
	_, err = io.Copy(entry, file)
	return err
}
