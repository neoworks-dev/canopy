package zoxide

import "strings"

// matchesKeywords applies zoxide's own rule, so a query that works in the shell
// works here: the keywords have to appear in the path in order, and the last one
// has to appear in the final path component. That last part is what makes "z dow"
// land on Downloads rather than on the first thing inside it.
func matchesKeywords(path string, keywords []string) bool {
	if len(keywords) == 0 {
		return true
	}
	lowered := strings.ToLower(path)
	cursor, matched := consumeLeadingKeywords(lowered, keywords)
	if !matched {
		return false
	}
	return lastKeywordIsInTheName(lowered, keywords[len(keywords)-1], cursor)
}

// consumeLeadingKeywords walks every keyword but the last, reporting how far
// along the path they took us.
func consumeLeadingKeywords(lowered string, keywords []string) (int, bool) {
	cursor := 0
	for _, keyword := range keywords[:len(keywords)-1] {
		offset := strings.Index(lowered[cursor:], keyword)
		if offset < 0 {
			return 0, false
		}
		cursor += offset + len(keyword)
	}
	return cursor, true
}

func lastKeywordIsInTheName(lowered string, keyword string, cursor int) bool {
	offset := strings.Index(lowered[cursor:], keyword)
	if offset < 0 {
		return false
	}
	return cursor+offset > strings.LastIndex(lowered, "/")
}

func splitKeywords(text string) []string {
	return strings.Fields(strings.ToLower(text))
}
