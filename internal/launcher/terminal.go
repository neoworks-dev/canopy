package launcher

import (
	"errors"
	"fmt"
	"os"
	"os/exec"
	"strings"
)

// terminalSchemeType is the association a desktop declares its default terminal
// under.
const terminalSchemeType = "x-scheme-handler/terminal"

// knownTerminals is the last resort for a system that declares no default.
var knownTerminals = []string{
	"ghostty",
	"kitty",
	"alacritty",
	"wezterm",
	"foot",
	"konsole",
	"gnome-terminal",
	"xfce4-terminal",
	"tilix",
	"terminator",
	"urxvt",
	"xterm",
}

// OpenTerminal starts a terminal in a directory. The terminal inherits its
// working directory from the process that starts it, which avoids every
// emulator's own spelling of a directory flag.
func OpenTerminal(directory string) error {
	err := requireDirectory(directory)
	if err != nil {
		return err
	}
	argv := terminalCommand()
	if len(argv) == 0 {
		return errors.New("no terminal emulator found")
	}
	return runDetachedIn(directory, argv[0], argv[1:]...)
}

func requireDirectory(directory string) error {
	info, err := os.Stat(directory)
	if err != nil {
		return err
	}
	if !info.IsDir() {
		return fmt.Errorf("%s is not a directory", directory)
	}
	return nil
}

// terminalCommand picks the terminal to start: the user's $TERMINAL first, then
// the desktop entry registered for the terminal scheme, then whichever known
// emulator is installed.
func terminalCommand() []string {
	fromEnvironment := installedCommand(os.Getenv("TERMINAL"))
	if fromEnvironment != nil {
		return fromEnvironment
	}
	fromDesktop := defaultTerminalEntry()
	if fromDesktop != nil {
		return fromDesktop
	}
	return firstInstalled(knownTerminals)
}

func defaultTerminalEntry() []string {
	for _, desktopID := range desktopIDsFor(terminalSchemeType) {
		argv := terminalArgv(desktopID)
		if argv != nil {
			return argv
		}
	}
	return nil
}

func terminalArgv(desktopID string) []string {
	entry, usable := loadEntry(desktopID)
	if !usable {
		return nil
	}
	return runnableCommand(splitExec(entry.exec))
}

// runnableCommand drops the desktop entry field codes, leaving the command line
// that starts the application with nothing to open.
func runnableCommand(fields []string) []string {
	argv := []string{}
	for _, field := range fields {
		argv = appendLiteralField(argv, field)
	}
	if len(argv) == 0 {
		return nil
	}
	if installedCommand(argv[0]) == nil {
		return nil
	}
	return argv
}

func appendLiteralField(argv []string, field string) []string {
	if strings.HasPrefix(field, "%") {
		return argv
	}
	return append(argv, field)
}

func firstInstalled(names []string) []string {
	for _, name := range names {
		found := installedCommand(name)
		if found != nil {
			return found
		}
	}
	return nil
}

func installedCommand(name string) []string {
	if name == "" {
		return nil
	}
	_, err := exec.LookPath(name)
	if err != nil {
		return nil
	}
	return []string{name}
}
