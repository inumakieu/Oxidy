oxidy.opt.relative_numbers = true;
oxidy.opt.natural_scroll = true;
oxidy.theme = "matugen";

// Hot reloading added
// Next is regex loading

/*
syntax("rs", || { 
    set_syntax("keywords", "\\b(let|pub|impl|use|fn|return|for|while|if|continue|test)\\b");
    set_syntax("comments", "^[^/]*(//.*)");
    set_syntax("functions", "\\s+([a-zA-Z_][a-zA-Z0-9_]*)\\(");
    set_syntax("literals", "\\b(String|i8|i16|i32|i64|u8|u16|u32|u64|usize|isize|Vec)\\b");
});
*/

theme("matugen", || {
    // Core syntax colors (linked to MaterialYou roles)
    set_color("namespace", "#cabeff");
    set_color("type", "#c9c4d0");
    set_color("class", "#c9c4d0");
    set_color("enum", "#c9c4d0");
    set_color("interface", "#c9c4d0");
    set_color("struct", "#c9c4d0");
    set_color("typeParameter", "#c9c3dc");
    set_color("parameter", "#e6e1e9");
    set_color("variable", "#e6e1e9");
    set_color("property", "#c9c4d0");
    set_color("enumMember", "#edb8cc");
    set_color("event", "#cabeff");
    set_color("function", "#cabeff");
    set_color("method", "#cabeff");
    set_color("macro", "#c9c3dc");
    set_color("keyword", "#c9c3dc");
    set_color("modifier", "#c9c3dc");
    set_color("comment", "#48454e");
    set_color("string", "#edb8cc");
    set_color("number", "#c9c4d0");
    set_color("regexp", "#c9c4d0");
    set_color("operator", "#c9c4d0");

    // UI accents (using container/surface tones)
    set_color("cursor", "#e6deff");
    set_color("line_number", "#48454e");
    set_color("selection_bg", "#483f77");
    set_color("background", "#141318");
    set_color("surface", "#1c1b20");
});

