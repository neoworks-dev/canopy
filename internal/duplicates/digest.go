package duplicates

import (
	"crypto/sha256"
	"encoding/hex"
	"hash"
	"io"
	"os"
)

// How much is read from each end of a file for its signature. Files that differ
// usually differ near one end or the other, so this settles most of them without
// reading what lies between.
const signatureLength = 4096

// signatureOf digests both ends of a file. One small enough to be covered by a
// single read is simply digested whole.
func signatureOf(path string, size int64) (string, error) {
	file, err := os.Open(path)
	if err != nil {
		return "", err
	}
	defer file.Close()
	digest := sha256.New()
	err = digestSection(digest, file, 0, signatureLength)
	if err != nil || size <= signatureLength {
		return digestOrError(digest, err)
	}
	err = digestSection(digest, file, size-signatureLength, signatureLength)
	return digestOrError(digest, err)
}

// contentsOf digests a whole file, which is what settles the files a matching
// signature leaves indistinguishable. The size is what the caller groups by
// rather than anything this needs.
func contentsOf(path string, _ int64) (string, error) {
	file, err := os.Open(path)
	if err != nil {
		return "", err
	}
	defer file.Close()
	digest := sha256.New()
	_, err = io.Copy(digest, file)
	return digestOrError(digest, err)
}

func digestSection(digest io.Writer, file *os.File, offset, length int64) error {
	_, err := io.Copy(digest, io.NewSectionReader(file, offset, length))
	return err
}

func digestOrError(digest hash.Hash, err error) (string, error) {
	if err != nil {
		return "", err
	}
	return hex.EncodeToString(digest.Sum(nil)), nil
}
