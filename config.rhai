oxidy.opt.relative_numbers = true;
oxidy.opt.natural_scroll = true;

// Hot reloading added
// Next is regex loading

syntax("rs", || { 
    set_syntax("keywords", "\\b(let|pub|impl|use|fn|return|for|while|if|continue|test)\\b");
    set_syntax("comments", "^[^/]*(//.*)");
    set_syntax("functions", "\\s+([a-zA-Z_][a-zA-Z0-9_]*)\\(");
    set_syntax("literals", "\\b(String|i8|i16|i32|i64|u8|u16|u32|u64|usize|isize|Vec)\\b");
});
