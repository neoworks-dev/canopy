/**
 * "when" clauses decide whether a keybinding or a command applies right now.
 * They are strings rather than predicates because a plugin contributes them
 * over RPC, where a function cannot cross, and because the settings file has to
 * be able to carry them.
 *
 * The grammar is a small subset of the one VS Code uses:
 *
 *     listFocused && selectionCount > 0
 *     mode == normal && !searchOpen
 *     viewMode == grid || viewMode == rows
 *
 * A bare identifier is true when its context value is truthy. On the right of a
 * comparison an identifier that the context does not define is taken as a plain
 * string, so `viewMode == grid` needs no quotes.
 */

export type WhenValue = string | number | boolean;
export type WhenContext = Record<string, WhenValue>;

type Token = { kind: "symbol" | "identifier" | "number" | "string"; text: string };

type Node =
	| { kind: "literal"; value: WhenValue }
	| { kind: "identifier"; name: string }
	| { kind: "not"; operand: Node }
	| { kind: "binary"; operator: string; left: Node; right: Node };

type Cursor = { tokens: Token[]; position: number };

const tokenPattern = /(&&|\|\||==|!=|>=|<=|[!()<>])|('[^']*'|"[^"]*")|(\d+(?:\.\d+)?)|([A-Za-z_][\w.]*)|(\s+)/y;

const comparisonOperators = ["==", "!=", ">=", "<=", ">", "<"];

// Clauses are evaluated on every key press and on every menu render, so each
// distinct string is parsed once.
const parsedClauses = new Map<string, Node | null>();

/** An empty clause always applies, which is what an unconditional binding is. */
export function evaluateWhen(clause: string, context: WhenContext): boolean {
	if (clause.trim() === "") {
		return true;
	}
	const node = parseClause(clause);
	if (!node) {
		return false;
	}
	return isTruthy(evaluate(node, context));
}

/** Reports whether a clause can be read at all, for validating stored settings. */
export function whenClauseIsValid(clause: string): boolean {
	if (clause.trim() === "") {
		return true;
	}
	return parseClause(clause) !== null;
}

function parseClause(clause: string): Node | null {
	const cached = parsedClauses.get(clause);
	if (cached !== undefined) {
		return cached;
	}
	const parsed = parseFully(clause);
	parsedClauses.set(clause, parsed);
	return parsed;
}

function parseFully(clause: string): Node | null {
	const tokens = tokenize(clause);
	if (!tokens) {
		return null;
	}
	const cursor: Cursor = { tokens, position: 0 };
	const node = parseOr(cursor);
	if (cursor.position !== tokens.length) {
		return null;
	}
	return node;
}

function tokenize(clause: string): Token[] | null {
	const scanner = new RegExp(tokenPattern.source, "y");
	const tokens: Token[] = [];
	while (scanner.lastIndex < clause.length) {
		const match = scanner.exec(clause);
		if (!match) {
			return null;
		}
		appendToken(tokens, match);
	}
	return tokens;
}

function appendToken(tokens: Token[], match: RegExpExecArray) {
	const [, symbol, quoted, number, identifier] = match;
	if (symbol) {
		tokens.push({ kind: "symbol", text: symbol });
	}
	appendValueToken(tokens, quoted, number, identifier);
}

function appendValueToken(
	tokens: Token[],
	quoted: string | undefined,
	number: string | undefined,
	identifier: string | undefined,
) {
	if (quoted) {
		tokens.push({ kind: "string", text: quoted.slice(1, -1) });
	}
	if (number) {
		tokens.push({ kind: "number", text: number });
	}
	if (identifier) {
		tokens.push({ kind: "identifier", text: identifier });
	}
}

function parseOr(cursor: Cursor): Node | null {
	let left = parseAnd(cursor);
	while (left && takeSymbol(cursor, "||")) {
		left = joinWith("||", left, parseAnd(cursor));
	}
	return left;
}

function parseAnd(cursor: Cursor): Node | null {
	let left = parseUnary(cursor);
	while (left && takeSymbol(cursor, "&&")) {
		left = joinWith("&&", left, parseUnary(cursor));
	}
	return left;
}

function parseUnary(cursor: Cursor): Node | null {
	if (!takeSymbol(cursor, "!")) {
		return parseComparison(cursor);
	}
	const operand = parseUnary(cursor);
	if (!operand) {
		return null;
	}
	return { kind: "not", operand };
}

function parseComparison(cursor: Cursor): Node | null {
	const left = parsePrimary(cursor);
	if (!left) {
		return null;
	}
	const operator = takeComparison(cursor);
	if (!operator) {
		return left;
	}
	return joinWith(operator, left, parsePrimary(cursor));
}

function parsePrimary(cursor: Cursor): Node | null {
	if (takeSymbol(cursor, "(")) {
		return parseGroup(cursor);
	}
	const token = takeToken(cursor);
	if (!token) {
		return null;
	}
	return nodeForToken(token);
}

function parseGroup(cursor: Cursor): Node | null {
	const inner = parseOr(cursor);
	if (!inner || !takeSymbol(cursor, ")")) {
		return null;
	}
	return inner;
}

function nodeForToken(token: Token): Node | null {
	if (token.kind === "identifier") {
		return { kind: "identifier", name: token.text };
	}
	if (token.kind === "string") {
		return { kind: "literal", value: token.text };
	}
	if (token.kind === "number") {
		return { kind: "literal", value: Number(token.text) };
	}
	return null;
}

function joinWith(operator: string, left: Node, right: Node | null): Node | null {
	if (!right) {
		return null;
	}
	return { kind: "binary", operator, left, right };
}

function takeSymbol(cursor: Cursor, text: string): boolean {
	const token = cursor.tokens[cursor.position];
	if (!token || token.kind !== "symbol" || token.text !== text) {
		return false;
	}
	cursor.position++;
	return true;
}

function takeComparison(cursor: Cursor): string {
	const token = cursor.tokens[cursor.position];
	if (!token || token.kind !== "symbol" || !comparisonOperators.includes(token.text)) {
		return "";
	}
	cursor.position++;
	return token.text;
}

function takeToken(cursor: Cursor): Token | null {
	const token = cursor.tokens[cursor.position];
	if (!token) {
		return null;
	}
	cursor.position++;
	return token;
}

function evaluate(node: Node, context: WhenContext): WhenValue {
	if (node.kind === "literal") {
		return node.value;
	}
	if (node.kind === "identifier") {
		return valueOf(node.name, context);
	}
	if (node.kind === "not") {
		return !isTruthy(evaluate(node.operand, context));
	}
	return evaluateBinary(node.operator, node.left, node.right, context);
}

function evaluateBinary(
	operator: string,
	left: Node,
	right: Node,
	context: WhenContext,
): WhenValue {
	if (operator === "&&") {
		return isTruthy(evaluate(left, context)) && isTruthy(evaluate(right, context));
	}
	if (operator === "||") {
		return isTruthy(evaluate(left, context)) || isTruthy(evaluate(right, context));
	}
	return compare(operator, evaluate(left, context), rightOperand(right, context));
}

/**
 * An identifier the context does not define reads as its own name, so a clause
 * can say `viewMode == grid` without quoting the value it is compared against.
 */
function rightOperand(node: Node, context: WhenContext): WhenValue {
	if (node.kind !== "identifier" || node.name in context) {
		return evaluate(node, context);
	}
	return node.name;
}

function valueOf(name: string, context: WhenContext): WhenValue {
	const value = context[name];
	if (value === undefined) {
		return false;
	}
	return value;
}

function compare(operator: string, left: WhenValue, right: WhenValue): boolean {
	if (operator === "==") {
		return String(left) === String(right);
	}
	if (operator === "!=") {
		return String(left) !== String(right);
	}
	return compareNumbers(operator, Number(left), Number(right));
}

function compareNumbers(operator: string, left: number, right: number): boolean {
	if (Number.isNaN(left) || Number.isNaN(right)) {
		return false;
	}
	return applyOrdering(operator, left, right);
}

function applyOrdering(operator: string, left: number, right: number): boolean {
	if (operator === ">") {
		return left > right;
	}
	if (operator === "<") {
		return left < right;
	}
	if (operator === ">=") {
		return left >= right;
	}
	return left <= right;
}

function isTruthy(value: WhenValue): boolean {
	if (typeof value === "string") {
		return value !== "";
	}
	return Boolean(value);
}
