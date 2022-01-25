// Package main implements a parser for HashiCorp's HCL configuration syntax.
package main

import (
	"os"

	"gopkg.in/alecthomas/kingpin.v2"

	"github.com/alecthomas/repr"

	"github.com/alecthomas/participle/v2"
	"github.com/alecthomas/participle/v2/lexer"
)

type Value struct {
	DateTime *string `@DateTime`
	Issue    *Issue  `| @@`
}

type Issue struct {
	Type *string ` @("github" | "gitlab")`
	URL  *string `@URL`
}

type Config struct {
	Action *string `@("TODO" | "FIXME")`
	Type   *string `@("before" | "after" | "ifclosed")`
	Value  *Value  ` @@`
}

var tomlLexer = lexer.MustSimple([]lexer.SimpleRule{
	// {"DateTime", `\d\d\d\d-\d\d-\d\dT\d\d:\d\d:\d\d(\.\d+)?(-\d\d:\d\d)?`},
	// ISO 8601
	{"DateTime", `\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(\.\d+)?(([+-]\d\d:\d\d)|Z)?`},
	{"URL", `[(http(s)?):\/\/(www\.)?a-zA-Z0-9@:%._\+~#=]{2,256}\.[a-z]{2,6}\b([-a-zA-Z0-9@:%_\+.~#?&//=]*)`},
	{"Ident", `[a-zA-Z_][a-zA-Z_0-9]*`},
	{"String", `"[^"]*"`},
	{"Int", `\d+`},
	{"Number", `[-+]?[.0-9]+\b`},
	{"comment", `#[^\n]+`},
	{"whitespace", `\s+`},
})

var parser = participle.MustBuild(&Config{}, participle.Lexer(

	tomlLexer), participle.Unquote())

func main() {
	kingpin.Parse()

	expr := &Config{}
	file, err := os.Open("./example")
	kingpin.FatalIfError(err, "")
	err = parser.Parse("", file, expr)
	kingpin.FatalIfError(err, "")

	repr.Println(expr)
}
