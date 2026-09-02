class Example {
  String text = "// TODO-OR-DIE: after 2020-01-01 inside a string";
  String multiline = """
      // TODO-OR-DIE: after 2020-01-01 inside a multiline string
      """;
  // TODO-OR-DIE: after 2099-01-01
  /* TODO-OR-DIE: after 2098-01-01
   * multiline comment
   */
  void example() {}
}
