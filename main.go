// Package main implements a parser for HashiCorp's HCL configuration syntax.
package main

import (
	"errors"
	"fmt"
	"io/ioutil"
	"log"
	"os"
	"path/filepath"
	"regexp"

	"gopkg.in/alecthomas/kingpin.v2"

	"github.com/alecthomas/repr"
	"github.com/iafan/cwalk"

	"github.com/alecthomas/participle/v2"
	"github.com/alecthomas/participle/v2/lexer"
	mapset "github.com/deckarep/golang-set"
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

var ignoredDirs = mapset.NewSet()
var ignoredFiles = mapset.NewSet()
var regex = regexp.MustCompile(`TODO[\w\s]*`)

func walkFunc(path string, d os.FileInfo, err error) error {
	// func walkFunc(path string, d fs.DirEntry, err error) error {
	if d.IsDir() {
		if ignoredDirs.Contains(d.Name()) {
			return filepath.SkipDir
		}
	} else {
		if ignoredFiles.Contains(d.Name()) {
			return nil
		}

		// file, err := os.Open(path)
		file, err := os.Open(filepath.Join(baseDir, path))
		defer file.Close()
		if err != nil {
			return errors.New(fmt.Sprintf("Error in file %s: %s", path, err))
		}

		b, err := ioutil.ReadAll(file)
		if err != nil {
			return errors.New(fmt.Sprintf("Error in file %s: %s", path, err))
		}
		matches := regex.FindAllString(string(b), -1)
		if len(matches) > 0 {
			fmt.Println("Found match in " + filepath.Join(baseDir, path))
			for _, match := range matches {
				fmt.Println(match)
			}
		}
	}

	return nil
}

var baseDir = "/home/reus/CENEVAL"

func main() {
	kingpin.Parse()

	ignoredDirs.Add(".terraform")
	ignoredDirs.Add(".git")
	ignoredDirs.Add("node_modules")

	ignoredFiles.Add(".terraform.lock.hcl")
	ignoredFiles.Add(".gitignore")

	expr := &Config{}
	file, err := os.Open("./example")
	kingpin.FatalIfError(err, "")
	err = parser.Parse("", file, expr)
	kingpin.FatalIfError(err, "")

	err = cwalk.Walk(baseDir, walkFunc)
	// err = filepath.WalkDir(baseDir, walkFunc)

	if err != nil {
		log.Fatalln(err)
	}

	repr.Println(expr)
}
