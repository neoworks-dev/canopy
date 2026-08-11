package launcher

import (
	"fmt"
	"net/url"
	"os/exec"
	"strings"
	"syscall"
)

// Open launches a path with the remembered override for its type, or with the
// system default when there is none.
func Open(path string) error {
	applicationID := overrideFor(MimeTypeOf(path))
	if applicationID != "" {
		return OpenWith(path, applicationID)
	}
	return runDetached("xdg-open", path)
}

// OpenWith launches a path with one specific desktop entry.
func OpenWith(path, applicationID string) error {
	entry, err := loadApplication(applicationID)
	if err != nil {
		return err
	}
	if entry.terminal {
		return fmt.Errorf("%s needs a terminal, which this explorer cannot provide yet", entry.name)
	}
	argv := buildCommand(entry.exec, path)
	if len(argv) == 0 {
		return fmt.Errorf("%s has no usable command line", applicationID)
	}
	return runDetached(argv[0], argv[1:]...)
}

// ApplicationsFor lists the desktop entries registered for a path's MIME type.
func ApplicationsFor(path string) []Application {
	applications := []Application{}
	for _, desktopID := range desktopIDsFor(MimeTypeOf(path)) {
		applications = appendApplication(applications, desktopID)
	}
	return applications
}

func appendApplication(applications []Application, desktopID string) []Application {
	entry, usable := loadEntry(desktopID)
	if !usable || entry.noDisplay {
		return applications
	}
	return append(applications, Application{
		ID:   desktopID,
		Name: displayName(entry, desktopID),
		Icon: entry.icon,
		Exec: entry.exec,
	})
}

func displayName(entry desktopEntry, desktopID string) string {
	if entry.name != "" {
		return entry.name
	}
	return strings.TrimSuffix(desktopID, ".desktop")
}

func loadApplication(desktopID string) (desktopEntry, error) {
	entry, usable := loadEntry(desktopID)
	if !usable {
		return desktopEntry{}, fmt.Errorf("no launchable desktop entry named %s", desktopID)
	}
	return entry, nil
}

func loadEntry(desktopID string) (desktopEntry, bool) {
	desktopPath := findDesktopFile(desktopID)
	if desktopPath == "" {
		return desktopEntry{}, false
	}
	return readDesktopEntry(desktopPath)
}

// buildCommand expands the desktop entry field codes. An entry that names no
// placeholder still gets the path appended, which is what launchers do.
func buildCommand(execLine, path string) []string {
	argv := []string{}
	substituted := false
	for _, field := range splitExec(execLine) {
		argv, substituted = appendExecField(argv, field, path, substituted)
	}
	if substituted {
		return argv
	}
	return append(argv, path)
}

func appendExecField(argv []string, field, path string, substituted bool) ([]string, bool) {
	if field == "%f" || field == "%F" {
		return append(argv, path), true
	}
	if field == "%u" || field == "%U" {
		return append(argv, fileURI(path)), true
	}
	if strings.HasPrefix(field, "%") {
		return argv, substituted
	}
	return append(argv, field), substituted
}

func fileURI(path string) string {
	return (&url.URL{Scheme: "file", Path: path}).String()
}

// execScanner splits an Exec line on unquoted spaces, the only quoting rule the
// desktop entry spec needs here.
type execScanner struct {
	fields  []string
	current strings.Builder
	quoted  bool
}

func splitExec(execLine string) []string {
	scanner := execScanner{fields: []string{}}
	for _, character := range execLine {
		scanner.consume(character)
	}
	scanner.flush()
	return scanner.fields
}

func (scanner *execScanner) consume(character rune) {
	if character == '"' {
		scanner.quoted = !scanner.quoted
		return
	}
	if character == ' ' && !scanner.quoted {
		scanner.flush()
		return
	}
	scanner.current.WriteRune(character)
}

func (scanner *execScanner) flush() {
	if scanner.current.Len() == 0 {
		return
	}
	scanner.fields = append(scanner.fields, scanner.current.String())
	scanner.current.Reset()
}

// runDetached starts the process in its own session so it outlives the explorer,
// and reaps it in the background rather than leaving a zombie.
func runDetached(name string, arguments ...string) error {
	_, err := exec.LookPath(name)
	if err != nil {
		return fmt.Errorf("%s is not installed", name)
	}
	command := exec.Command(name, arguments...)
	command.SysProcAttr = &syscall.SysProcAttr{Setsid: true}
	err = command.Start()
	if err != nil {
		return err
	}
	go command.Wait()
	return nil
}
