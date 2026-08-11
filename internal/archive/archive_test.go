package archive

import (
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"
)

func TestFormatIsReadFromTheName(t *testing.T) {
	cases := map[string]Format{
		"photos.zip":       FormatZip,
		"PHOTOS.ZIP":       FormatZip,
		"backup.tar":       FormatTar,
		"backup.tar.gz":    FormatTarGzip,
		"backup.tgz":       FormatTarGzip,
		"backup.tar.bz2":   FormatTarBzip2,
		"notes.txt":        FormatUnknown,
		"archive.zip.part": FormatUnknown,
	}
	for name, wanted := range cases {
		if FormatOf(name) != wanted {
			t.Fatalf("FormatOf(%q) = %q, want %q", name, FormatOf(name), wanted)
		}
	}
}

// A .tar.gz must not be read as the .gz it also ends in.
func TestTheLongerSuffixWins(t *testing.T) {
	if FormatOf("backup.tar.gz") != FormatTarGzip {
		t.Fatal("a compressed tar is a tar rather than whatever compressed it")
	}
}

func TestAPackedTreeComesBackWhole(t *testing.T) {
	for _, name := range []string{"bundle.zip", "bundle.tar", "bundle.tar.gz"} {
		packAndCheck(t, name)
	}
}

func packAndCheck(t *testing.T, archiveName string) {
	t.Helper()
	root := t.TempDir()
	source := filepath.Join(root, "project")
	mkdir(t, filepath.Join(source, "nested"))
	writeFile(t, filepath.Join(source, "top.txt"), "top")
	writeFile(t, filepath.Join(source, "nested", "deep.txt"), "deep")

	archivePath := filepath.Join(root, archiveName)
	err := Create(archivePath, []string{source}, func(string, int64) error { return nil })
	if err != nil {
		t.Fatalf("Create %s: %v", archiveName, err)
	}

	contents := readMembers(t, archivePath)
	if contents["project/top.txt"] != "top" {
		t.Fatalf("%s: top.txt = %q", archiveName, contents["project/top.txt"])
	}
	if contents["project/nested/deep.txt"] != "deep" {
		t.Fatalf("%s: deep.txt = %q", archiveName, contents["project/nested/deep.txt"])
	}
}

func TestListingReportsTheFoldersAsWellAsTheFiles(t *testing.T) {
	root := t.TempDir()
	source := filepath.Join(root, "project")
	mkdir(t, filepath.Join(source, "nested"))
	writeFile(t, filepath.Join(source, "nested", "deep.txt"), "deep")
	archivePath := filepath.Join(root, "bundle.zip")
	err := Create(archivePath, []string{source}, func(string, int64) error { return nil })
	if err != nil {
		t.Fatalf("Create: %v", err)
	}

	members, err := List(archivePath)
	if err != nil {
		t.Fatalf("List: %v", err)
	}

	names := memberNames(members)
	if !sameStrings(names, []string{"project", "project/nested", "project/nested/deep.txt"}) {
		t.Fatalf("names = %v", names)
	}
	if !isDirectory(members, "project/nested") {
		t.Fatal("a folder inside an archive must be reported as one")
	}
}

func TestAVisitThatFailsStopsTheWalk(t *testing.T) {
	root := t.TempDir()
	source := filepath.Join(root, "project")
	mkdir(t, source)
	writeFile(t, filepath.Join(source, "one.txt"), "one")
	writeFile(t, filepath.Join(source, "two.txt"), "two")
	archivePath := filepath.Join(root, "bundle.zip")
	err := Create(archivePath, []string{source}, func(string, int64) error { return nil })
	if err != nil {
		t.Fatalf("Create: %v", err)
	}

	stopped := io.ErrUnexpectedEOF
	visited := 0
	err = Walk(archivePath, func(Member, io.Reader) error {
		visited++
		return stopped
	})

	if err != stopped {
		t.Fatalf("Walk = %v, want the error the visit reported", err)
	}
	if visited != 1 {
		t.Fatalf("visited %d members, want the walk to stop at the first", visited)
	}
}

func TestPackingAFormatThatCannotBeWrittenIsRefused(t *testing.T) {
	root := t.TempDir()
	writeFile(t, filepath.Join(root, "note.txt"), "body")

	err := Create(filepath.Join(root, "bundle.tar.bz2"), []string{filepath.Join(root, "note.txt")},
		func(string, int64) error { return nil })

	if err != ErrCannotWriteFormat {
		t.Fatalf("Create of a bzip2 tar = %v, want ErrCannotWriteFormat", err)
	}
}

func TestOpeningSomethingThatIsNotAnArchiveIsRefused(t *testing.T) {
	path := filepath.Join(t.TempDir(), "note.txt")
	writeFile(t, path, "body")

	_, err := List(path)

	if err != ErrUnsupportedFormat {
		t.Fatalf("List of a plain file = %v, want ErrUnsupportedFormat", err)
	}
}

func TestAMemberPointingOutsideTheFolderIsRefused(t *testing.T) {
	escaping := []string{"../outside.txt", "/etc/passwd", "nested/../../outside.txt"}
	for _, name := range escaping {
		_, err := DestinationFor("/home/moritz/unpacked", Member{Name: name})
		if err != ErrEscapingMember {
			t.Fatalf("DestinationFor(%q) = %v, want ErrEscapingMember", name, err)
		}
	}
}

func TestAMemberStayingInsideLandsUnderTheFolder(t *testing.T) {
	target, err := DestinationFor("/home/moritz/unpacked", Member{Name: "project/top.txt"})
	if err != nil {
		t.Fatalf("DestinationFor: %v", err)
	}
	if target != "/home/moritz/unpacked/project/top.txt" {
		t.Fatalf("target = %q", target)
	}
}

func readMembers(t *testing.T, archivePath string) map[string]string {
	t.Helper()
	contents := map[string]string{}
	err := Walk(archivePath, func(member Member, reader io.Reader) error {
		if member.IsDir {
			return nil
		}
		body, err := io.ReadAll(reader)
		contents[member.Name] = string(body)
		return err
	})
	if err != nil {
		t.Fatalf("Walk %s: %v", archivePath, err)
	}
	return contents
}

func memberNames(members []Member) []string {
	names := make([]string, 0, len(members))
	for _, member := range members {
		names = append(names, member.Name)
	}
	return names
}

func isDirectory(members []Member, name string) bool {
	for _, member := range members {
		if member.Name == name {
			return member.IsDir
		}
	}
	return false
}

func sameStrings(found []string, wanted []string) bool {
	sort.Strings(found)
	sort.Strings(wanted)
	return strings.Join(found, ",") == strings.Join(wanted, ",")
}

func writeFile(t *testing.T, path, contents string) {
	t.Helper()
	err := os.WriteFile(path, []byte(contents), 0o644)
	if err != nil {
		t.Fatalf("write %s: %v", path, err)
	}
}

func mkdir(t *testing.T, path string) {
	t.Helper()
	err := os.MkdirAll(path, 0o755)
	if err != nil {
		t.Fatalf("mkdir %s: %v", path, err)
	}
}
