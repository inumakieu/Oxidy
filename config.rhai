
oxidy.opt = #{
    relative_numbers: true,
    tab_size: 4
};

oxidy.theme = "oldworld";

oxidy.themes = #{
    oldworld: #{
        Background:      "#161617",
        Foreground:      "#c9c7cd",
        Comment:         "#8b8693",

        Namespace:       "#ea83a5",
        Type:            "#e6b99d",
        Class:           "#e6b99d",
        Struct:          "#e6b99d",
        Enum:            "#e6b99d",
        Interface:       "#e6b99d",
        TypeParameter:   "#9f9ca6",

        Variable:        "#c9c7cd",
        Parameter:       "#b4b1ba",
        Property:        "#d8b38b",
        EnumMember:      "#d8b38b",

        Function:        "#92a2d5",
        Method:          "#92a2d5",
        Macro:           "#ea83a5",
        Event:           "#85b5ba",

        Keyword:         "#aca1cf",
        Modifier:        "#aca1cf",
        Operator:        "#e3dcca",

        String:          "#90b99f",
        Number:          "#e29eca",
        Regexp:          "#e29eca"
    };
};

oxidy.lsps = #{
    rs: #{
        extensions: ["rs"],
        command: "rust-analyzer",
        args: [],
    }
}
