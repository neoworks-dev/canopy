// Package clipboard exchanges file selections with other applications through
// the text/uri-list target, which is what file managers agree on.
package clipboard

import (
	"errors"
	"net/url"
	"os/exec"
	"strings"
)

const uriListType = "text/uri-list"

var ErrNoClipboardTool = errors.New("neither wl-copy nor xclip is installed")

// writers and readers are tried in order, so a Wayland session picks wl-clipboard
// and an X11 one falls through to xclip.
var writers = []command{
	{name: "wl-copy", arguments: []string{"--type", uriListType}},
	{name: "xclip", arguments: []string{"-selection", "clipboard", "-t", uriListType, "-i"}},
}

var readers = []command{
	{name: "wl-paste", arguments: []string{"--no-newline", "--type", uriListType}},
	{name: "xclip", arguments: []string{"-selection", "clipboard", "-t", uriListType, "-o"}},
}

type command struct {
	name      string
	arguments []string
}

// WritePaths publishes a selection so other applications can paste it.
func WritePaths(paths []string) error {
	payload := strings.Join(toURIs(paths), "\r\n") + "\r\n"
	for _, writer := range writers {
		err := writer.feed(payload)
		if err == nil {
			return nil
		}
	}
	return ErrNoClipboardTool
}

// ReadPaths returns the file paths another application put on the clipboard.
func ReadPaths() ([]string, error) {
	for _, reader := range readers {
		output, err := reader.capture()
		if err == nil {
			return toPaths(output), nil
		}
	}
	return nil, ErrNoClipboardTool
}

func (helper command) feed(payload string) error {
	_, err := exec.LookPath(helper.name)
	if err != nil {
		return err
	}
	process := exec.Command(helper.name, helper.arguments...)
	process.Stdin = strings.NewReader(payload)
	return process.Run()
}

func (helper command) capture() (string, error) {
	_, err := exec.LookPath(helper.name)
	if err != nil {
		return "", err
	}
	output, err := exec.Command(helper.name, helper.arguments...).Output()
	if err != nil {
		return "", err
	}
	return string(output), nil
}

func toURIs(paths []string) []string {
	uris := make([]string, 0, len(paths))
	for _, path := range paths {
		uris = append(uris, (&url.URL{Scheme: "file", Path: path}).String())
	}
	return uris
}

// toPaths keeps only local files: a uri-list may carry comments and remote URIs
// this explorer cannot act on.
func toPaths(payload string) []string {
	paths := []string{}
	for _, line := range strings.Split(payload, "\n") {
		paths = appendLocalPath(paths, strings.TrimSpace(line))
	}
	return paths
}

func appendLocalPath(paths []string, line string) []string {
	if line == "" || strings.HasPrefix(line, "#") {
		return paths
	}
	parsed, err := url.Parse(line)
	if err != nil || parsed.Scheme != "file" {
		return paths
	}
	return append(paths, parsed.Path)
}
