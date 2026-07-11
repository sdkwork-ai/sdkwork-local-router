type Json5Primitive = string | number | boolean | null;
type Json5Value = Json5Primitive | Json5Value[] | { [key: string]: Json5Value };

class Json5Parser {
  private readonly input: string;
  private index = 0;

  constructor(input: string) {
    this.input = input;
  }

  parse(): Json5Value {
    this.skipWhitespaceAndComments();
    const value = this.parseValue();
    this.skipWhitespaceAndComments();
    if (this.index < this.input.length) {
      this.throwSyntaxError("Unexpected trailing content");
    }
    return value;
  }

  private parseValue(): Json5Value {
    this.skipWhitespaceAndComments();
    const current = this.peek();

    if (current === "{") {
      return this.parseObject();
    }
    if (current === "[") {
      return this.parseArray();
    }
    if (current === '"' || current === "'") {
      return this.parseString();
    }
    if (current === "t") {
      this.expectKeyword("true");
      return true;
    }
    if (current === "f") {
      this.expectKeyword("false");
      return false;
    }
    if (current === "n") {
      this.expectKeyword("null");
      return null;
    }

    return this.parseNumber();
  }

  private parseObject(): { [key: string]: Json5Value } {
    this.expect("{");
    const result: { [key: string]: Json5Value } = {};
    this.skipWhitespaceAndComments();

    if (this.peek() === "}") {
      this.index += 1;
      return result;
    }

    while (this.index < this.input.length) {
      const key = this.parseObjectKey();
      this.skipWhitespaceAndComments();
      this.expect(":");
      result[key] = this.parseValue();
      this.skipWhitespaceAndComments();

      const current = this.peek();
      if (current === ",") {
        this.index += 1;
        this.skipWhitespaceAndComments();
        if (this.peek() === "}") {
          this.index += 1;
          return result;
        }
        continue;
      }
      if (current === "}") {
        this.index += 1;
        return result;
      }

      this.throwSyntaxError('Expected "," or "}" in object literal');
    }

    this.throwSyntaxError("Unterminated object literal");
  }

  private parseArray(): Json5Value[] {
    this.expect("[");
    const result: Json5Value[] = [];
    this.skipWhitespaceAndComments();

    if (this.peek() === "]") {
      this.index += 1;
      return result;
    }

    while (this.index < this.input.length) {
      result.push(this.parseValue());
      this.skipWhitespaceAndComments();

      const current = this.peek();
      if (current === ",") {
        this.index += 1;
        this.skipWhitespaceAndComments();
        if (this.peek() === "]") {
          this.index += 1;
          return result;
        }
        continue;
      }
      if (current === "]") {
        this.index += 1;
        return result;
      }

      this.throwSyntaxError('Expected "," or "]" in array literal');
    }

    this.throwSyntaxError("Unterminated array literal");
  }

  private parseObjectKey(): string {
    this.skipWhitespaceAndComments();
    const current = this.peek();
    if (current === '"' || current === "'") {
      return this.parseString();
    }

    return this.parseIdentifier();
  }

  private parseIdentifier(): string {
    const fromPos = this.index;
    const first = this.peek();
    if (!first || !/[A-Za-z_$]/.test(first)) {
      this.throwSyntaxError("Expected an object key");
    }

    this.index += 1;
    while (this.index < this.input.length) {
      const current = this.peek();
      if (!current || !/[A-Za-z0-9_$]/.test(current)) {
        break;
      }
      this.index += 1;
    }

    return this.input.slice(fromPos, this.index);
  }

  private parseString(): string {
    const quote = this.peek();
    if (quote !== '"' && quote !== "'") {
      this.throwSyntaxError("Expected a string literal");
    }
    this.index += 1;

    let result = "";
    while (this.index < this.input.length) {
      const current = this.input[this.index] ?? "";
      this.index += 1;

      if (current === quote) {
        return result;
      }
      if (current === "\\") {
        result += this.parseEscapedCharacter();
        continue;
      }
      result += current;
    }

    this.throwSyntaxError("Unterminated string literal");
  }

  private parseEscapedCharacter(): string {
    const current = this.input[this.index] ?? "";
    this.index += 1;

    switch (current) {
      case "'":
      case '"':
      case "\\":
      case "/":
        return current;
      case "b":
        return "\b";
      case "f":
        return "\f";
      case "n":
        return "\n";
      case "r":
        return "\r";
      case "t":
        return "\t";
      case "v":
        return "\v";
      case "0":
        return "\0";
      case "x": {
        const value = this.readHexSequence(2);
        return String.fromCharCode(value);
      }
      case "u": {
        const value = this.readHexSequence(4);
        return String.fromCharCode(value);
      }
      case "\r":
        if (this.peek() === "\n") {
          this.index += 1;
        }
        return "";
      case "\n":
        return "";
      default:
        return current;
    }
  }

  private readHexSequence(length: number): number {
    const sequence = this.input.slice(this.index, this.index + length);
    if (!/^[0-9A-Fa-f]+$/.test(sequence) || sequence.length !== length) {
      this.throwSyntaxError("Invalid hexadecimal escape sequence");
    }
    this.index += length;
    return Number.parseInt(sequence, 16);
  }

  private parseNumber(): number {
    const fromPos = this.index;

    if (this.peek() === "+" || this.peek() === "-") {
      this.index += 1;
    }

    if (this.peek() === "I" && this.input.slice(this.index, this.index + 8) === "Infinity") {
      this.index += 8;
      return this.input[fromPos] === "-" ? -Infinity : Infinity;
    }
    if (this.peek() === "N" && this.input.slice(this.index, this.index + 3) === "NaN") {
      this.index += 3;
      return Number.NaN;
    }

    if (this.peek() === "0" && (this.peek(1) === "x" || this.peek(1) === "X")) {
      this.index += 2;
      const digitsStart = this.index;
      while (/[0-9A-Fa-f]/.test(this.peek() ?? "")) {
        this.index += 1;
      }
      if (digitsStart === this.index) {
        this.throwSyntaxError("Invalid hexadecimal number");
      }
      return Number.parseInt(this.input.slice(fromPos, this.index), 16);
    }

    let consumedDigit = false;
    while (/[0-9]/.test(this.peek() ?? "")) {
      consumedDigit = true;
      this.index += 1;
    }

    if (this.peek() === ".") {
      this.index += 1;
      while (/[0-9]/.test(this.peek() ?? "")) {
        consumedDigit = true;
        this.index += 1;
      }
    }

    if (this.peek() === "e" || this.peek() === "E") {
      const exponentStart = this.index;
      this.index += 1;
      if (this.peek() === "+" || this.peek() === "-") {
        this.index += 1;
      }
      const digitsStart = this.index;
      while (/[0-9]/.test(this.peek() ?? "")) {
        this.index += 1;
      }
      if (digitsStart === this.index) {
        this.index = exponentStart;
      } else {
        consumedDigit = true;
      }
    }

    if (!consumedDigit) {
      this.throwSyntaxError("Expected a JSON value");
    }

    const token = this.input.slice(fromPos, this.index);
    const value = Number(token);
    if (Number.isNaN(value)) {
      this.throwSyntaxError(`Invalid number literal "${token}"`);
    }
    return value;
  }

  private expectKeyword(keyword: string) {
    if (this.input.slice(this.index, this.index + keyword.length) !== keyword) {
      this.throwSyntaxError(`Expected "${keyword}"`);
    }
    this.index += keyword.length;
  }

  private expect(token: string) {
    if (this.peek() !== token) {
      this.throwSyntaxError(`Expected "${token}"`);
    }
    this.index += 1;
  }

  private skipWhitespaceAndComments() {
    while (this.index < this.input.length) {
      const current = this.peek();
      const next = this.peek(1);

      if (current && /\s/.test(current)) {
        this.index += 1;
        continue;
      }

      if (current === "/" && next === "/") {
        this.index += 2;
        while (this.index < this.input.length && this.peek() !== "\n") {
          this.index += 1;
        }
        continue;
      }

      if (current === "/" && next === "*") {
        this.index += 2;
        while (this.index < this.input.length) {
          if (this.peek() === "*" && this.peek(1) === "/") {
            this.index += 2;
            break;
          }
          this.index += 1;
        }
        continue;
      }

      break;
    }
  }

  private peek(offset = 0): string | undefined {
    return this.input[this.index + offset];
  }

  private throwSyntaxError(message: string): never {
    throw new SyntaxError(`${message} at position ${this.index}`);
  }
}

export function parseJson5<T = unknown>(input: string): T {
  return new Json5Parser(input).parse() as T;
}

export function stringifyJson5(value: unknown, space?: string | number): string {
  const indentUnit =
    typeof space === "number"
      ? " ".repeat(Math.max(0, space))
      : typeof space === "string"
        ? space
        : "";
  return stringifyValue(value as Json5Value, indentUnit, 0);
}

function stringifyValue(value: Json5Value, indentUnit: string, depth: number): string {
  if (value === null) {
    return "null";
  }
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "number") {
    return Number.isFinite(value) ? String(value) : "null";
  }
  if (typeof value === "boolean") {
    return value ? "true" : "false";
  }
  if (Array.isArray(value)) {
    if (value.length === 0) {
      return "[]";
    }
    if (!indentUnit) {
      return `[${value.map((entry) => stringifyValue(entry, indentUnit, depth + 1)).join(", ")}]`;
    }
    const indent = indentUnit.repeat(depth + 1);
    const closingIndent = indentUnit.repeat(depth);
    return `[\n${value
      .map((entry) => `${indent}${stringifyValue(entry, indentUnit, depth + 1)}`)
      .join(",\n")}\n${closingIndent}]`;
  }

  const entries = Object.entries(value).filter(([, entry]) => entry !== undefined);
  if (entries.length === 0) {
    return "{}";
  }
  if (!indentUnit) {
    return `{ ${entries
      .map(([key, entry]) => `${formatObjectKey(key)}: ${stringifyValue(entry, indentUnit, depth + 1)}`)
      .join(", ")} }`;
  }

  const indent = indentUnit.repeat(depth + 1);
  const closingIndent = indentUnit.repeat(depth);
  return `{\n${entries
    .map(
      ([key, entry]) =>
        `${indent}${formatObjectKey(key)}: ${stringifyValue(entry, indentUnit, depth + 1)}`,
    )
    .join(",\n")}\n${closingIndent}}`;
}

function formatObjectKey(key: string): string {
  return /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(key) ? key : JSON.stringify(key);
}
