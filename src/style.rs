// GRUVCOLR         HEX       RELATV ALIAS   TERMCOLOR      RGB           ITERM RGB     OSX HEX
// --------------   -------   ------------   ------------   -----------   -----------   -------
// dark0_hard       #1d2021   [   ]  [   ]   234 [h0][  ]    29- 32- 33    22- 24- 25   #161819
// dark0            #282828   [bg0]  [fg0]   235 [ 0][  ]    40- 40- 40    30- 30- 30   #1e1e1e
// dark0_soft       #32302f   [   ]  [   ]   236 [s0][  ]    50- 48- 47    38- 36- 35   #262423
// dark1            #3c3836   [bg1]  [fg1]   237 [  ][15]    60- 56- 54    46- 42- 41   #2e2a29
// dark2            #504945   [bg2]  [fg2]   239 [  ][  ]    80- 73- 69    63- 57- 53   #3f3935
// dark3            #665c54   [bg3]  [fg3]   241 [  ][  ]   102- 92- 84    83- 74- 66   #534a42
// dark4            #7c6f64   [bg4]  [fg4]   243 [  ][ 7]   124-111-100   104- 92- 81   #685c51
//
// gray_245         #928374   [gray] [   ]   245 [ 8][  ]   146-131-116   127-112- 97   #7f7061
// gray_244         #928374   [   ] [gray]   244 [  ][ 8]   146-131-116   127-112- 97   #7f7061
//
// light0_hard      #f9f5d7   [   ]  [   ]   230 [  ][h0]   249-245-215   248-244-205   #f8f4cd
// light0           #fbf1c7   [fg0]  [bg0]   229 [  ][ 0]   251-241-199   250-238-187   #faeebb
// light0_soft      #f2e5bc   [   ]  [   ]   228 [  ][s0]   242-229-188   239-223-174   #efdfae
// light1           #ebdbb2   [fg1]  [bg1]   223 [15][  ]   235-219-178   230-212-163   #e6d4a3
// light2           #d5c4a1   [fg2]  [bg2]   250 [  ][  ]   213-196-161   203-184-144   #cbb890
// light3           #bdae93   [fg3]  [bg3]   248 [  ][  ]   189-174-147   175-159-129   #af9f81
// light4           #a89984   [fg4]  [bg4]   246 [ 7][  ]   168-153-132   151-135-113   #978771
//
// bright_red       #fb4934   [red]   [  ]   167 [ 9][  ]   251- 73- 52   247- 48- 40   #f73028
// bright_green     #b8bb26   [green] [  ]   142 [10][  ]   184-187- 38   170-176- 30   #aab01e
// bright_yellow    #fabd2f   [yellow][  ]   214 [11][  ]   250-189- 47   247-177- 37   #f7b125
// bright_blue      #83a598   [blue]  [  ]   109 [12][  ]   131-165-152   113-149-134   #719586
// bright_purple    #d3869b   [purple][  ]   175 [13][  ]   211-134-155   199-112-137   #c77089
// bright_aqua      #8ec07c   [aqua]  [  ]   108 [14][  ]   142-192-124   125-182-105   #7db669
// bright_orange    #fe8019   [orange][  ]   208 [  ][  ]   254-128- 25   251-106- 22   #fb6a16
//
// neutral_red      #cc241d   [   ]  [   ]   124 [ 1][ 1]   204- 36- 29   190- 15- 23   #be0f17
// neutral_green    #98971a   [   ]  [   ]   106 [ 2][ 2]   152-151- 26   134-135- 21   #868715
// neutral_yellow   #d79921   [   ]  [   ]   172 [ 3][ 3]   215-153- 33   204-136- 26   #cc881a
// neutral_blue     #458588   [   ]  [   ]    66 [ 4][ 4]    69-133-136    55-115-117   #377375
// neutral_purple   #b16286   [   ]  [   ]   132 [ 5][ 5]   177- 98-134   160- 75-115   #a04b73
// neutral_aqua     #689d6a   [   ]  [   ]    72 [ 6][ 6]   104-157-106    87-142- 87   #578e57
// neutral_orange   #d65d0e   [   ]  [   ]   166 [  ][  ]   214- 93- 14   202- 72- 14   #ca480e
//
// faded_red        #9d0006   [   ]   [red]   88 [  ][ 9]   157-  0-  6   137-  0-  9   #890009
// faded_green      #79740e   [   ] [green]  100 [  ][10]   121-116- 14   102- 98- 13   #66620d
// faded_yellow     #b57614   [   ][yellow]  136 [  ][11]   181-118- 20   165- 99- 17   #a56311
// faded_blue       #076678   [   ]  [blue]   24 [  ][12]     7-102-120    14- 83-101   #0e5365
// faded_purple     #8f3f71   [   ][purple]   96 [  ][13]   143- 63-113   123- 43- 94   #7b2b5e
// faded_aqua       #427b58   [   ]  [aqua]   66 [  ][14]    66-123- 88    53-106- 70   #356a46
// faded_orange     #af3a03   [   ][orange]  130 [  ][  ]   175- 58-  3   157- 40-  7   #9d2807

use egui::Color32;

#[allow(dead_code)]
pub struct Theme {
    pub dark0_hard: Color32,
    pub dark0: Color32,
    pub dark0_soft: Color32,
    pub dark1: Color32,
    pub dark2: Color32,
    pub dark3: Color32,
    pub dark4: Color32,

    pub gray_245: Color32,
    pub gray_244: Color32,

    pub light0_hard: Color32,
    pub light0: Color32,
    pub light0_soft: Color32,
    pub light1: Color32,
    pub light2: Color32,
    pub light3: Color32,
    pub light4: Color32,

    pub bright_red: Color32,
    pub bright_green: Color32,
    pub bright_yellow: Color32,
    pub bright_blue: Color32,
    pub bright_purple: Color32,
    pub bright_aqua: Color32,
    pub bright_orange: Color32,

    pub neutral_red: Color32,
    pub neutral_green: Color32,
    pub neutral_yellow: Color32,
    pub neutral_blue: Color32,
    pub neutral_purple: Color32,
    pub neutral_aqua: Color32,
    pub neutral_orange: Color32,

    pub faded_red: Color32,
    pub faded_green: Color32,
    pub faded_yellow: Color32,
    pub faded_blue: Color32,
    pub faded_purple: Color32,
    pub faded_aqua: Color32,
    pub faded_orange: Color32,
}

pub const GRUVBOX: Theme = Theme {
    dark0_hard: Color32::from_rgb(29, 32, 33),
    dark0: Color32::from_rgb(40, 40, 40),
    dark0_soft: Color32::from_rgb(50, 48, 47),
    dark1: Color32::from_rgb(60, 56, 54),
    dark2: Color32::from_rgb(80, 73, 69),
    dark3: Color32::from_rgb(102, 92, 84),
    dark4: Color32::from_rgb(124, 111, 100),

    gray_245: Color32::from_rgb(146, 131, 116),
    gray_244: Color32::from_rgb(146, 131, 116),

    light0_hard: Color32::from_rgb(249, 245, 215),
    light0: Color32::from_rgb(251, 241, 199),
    light0_soft: Color32::from_rgb(242, 229, 188),
    light1: Color32::from_rgb(235, 219, 178),
    light2: Color32::from_rgb(213, 196, 161),
    light3: Color32::from_rgb(189, 174, 147),
    light4: Color32::from_rgb(168, 153, 132),

    bright_red: Color32::from_rgb(251, 73, 52),
    bright_green: Color32::from_rgb(184, 187, 38),
    bright_yellow: Color32::from_rgb(250, 189, 47),
    bright_blue: Color32::from_rgb(131, 165, 152),
    bright_purple: Color32::from_rgb(211, 134, 155),
    bright_aqua: Color32::from_rgb(142, 192, 124),
    bright_orange: Color32::from_rgb(254, 128, 25),

    neutral_red: Color32::from_rgb(204, 36, 29),
    neutral_green: Color32::from_rgb(152, 151, 26),
    neutral_yellow: Color32::from_rgb(215, 153, 33),
    neutral_blue: Color32::from_rgb(69, 133, 136),
    neutral_purple: Color32::from_rgb(177, 98, 134),
    neutral_aqua: Color32::from_rgb(104, 157, 106),
    neutral_orange: Color32::from_rgb(214, 93, 14),

    faded_red: Color32::from_rgb(157, 0, 6),
    faded_green: Color32::from_rgb(121, 116, 14),
    faded_yellow: Color32::from_rgb(181, 118, 20),
    faded_blue: Color32::from_rgb(7, 102, 120),
    faded_purple: Color32::from_rgb(143, 63, 113),
    faded_aqua: Color32::from_rgb(66, 123, 88),
    faded_orange: Color32::from_rgb(175, 58, 3),
};

fn set_widget_style(
    style: &mut egui::style::WidgetVisuals,
    fg: Color32,
    bg: Color32,
    weak: Color32,
    frame: Color32,
) {
    style.bg_fill = bg;
    style.weak_bg_fill = weak;
    style.bg_stroke.color = frame;
    style.fg_stroke.color = fg;
}

fn set_light_theme(style: &mut egui::style::Style) {
    const BG: Color32 = GRUVBOX.light0;
    const FG: Color32 = GRUVBOX.dark1;
    const ACCENT: Color32 = GRUVBOX.faded_green;
    set_widget_style(
        &mut style.visuals.widgets.noninteractive,
        GRUVBOX.dark3,
        BG,
        GRUVBOX.light1,
        GRUVBOX.light4,
    );
    set_widget_style(
        &mut style.visuals.widgets.inactive,
        GRUVBOX.dark2,
        BG,
        GRUVBOX.light1,
        GRUVBOX.light3,
    );
    set_widget_style(
        &mut style.visuals.widgets.hovered,
        GRUVBOX.dark1,
        BG,
        GRUVBOX.light2,
        GRUVBOX.light3,
    );
    set_widget_style(
        &mut style.visuals.widgets.active,
        ACCENT,
        BG,
        GRUVBOX.light1,
        GRUVBOX.light3,
    );

    style.visuals.hyperlink_color = GRUVBOX.bright_blue;
    style.visuals.warn_fg_color = GRUVBOX.neutral_orange;
    style.visuals.error_fg_color = GRUVBOX.neutral_red;
    style.visuals.extreme_bg_color = GRUVBOX.light0_hard;
    style.visuals.faint_bg_color = GRUVBOX.light0_soft;
    style.visuals.window_fill = BG;
    style.visuals.panel_fill = BG;
    style.visuals.window_stroke.color = FG;

    style.visuals.selection.bg_fill = GRUVBOX.light4;
    style.visuals.selection.stroke.color = ACCENT;
    style.visuals.text_cursor.stroke.color = ACCENT;
    style.visuals.window_shadow.color = GRUVBOX.dark0_soft;
    style.visuals.popup_shadow.color = GRUVBOX.dark0_soft;
}

fn set_dark_theme(style: &mut egui::style::Style) {
    const BG: Color32 = GRUVBOX.dark0;
    const FG: Color32 = GRUVBOX.light1;
    const ACCENT: Color32 = GRUVBOX.neutral_green;

    set_widget_style(
        &mut style.visuals.widgets.noninteractive,
        GRUVBOX.light3,
        BG,
        GRUVBOX.dark1,
        GRUVBOX.dark4,
    );
    set_widget_style(
        &mut style.visuals.widgets.inactive,
        GRUVBOX.light2,
        BG,
        GRUVBOX.dark1,
        GRUVBOX.dark3,
    );
    set_widget_style(
        &mut style.visuals.widgets.hovered,
        GRUVBOX.light1,
        BG,
        GRUVBOX.dark2,
        GRUVBOX.dark3,
    );
    set_widget_style(
        &mut style.visuals.widgets.active,
        ACCENT,
        BG,
        GRUVBOX.dark1,
        GRUVBOX.dark3,
    );

    style.visuals.hyperlink_color = GRUVBOX.bright_blue;
    style.visuals.warn_fg_color = GRUVBOX.neutral_orange;
    style.visuals.error_fg_color = GRUVBOX.neutral_red;
    style.visuals.extreme_bg_color = GRUVBOX.dark0_hard;
    style.visuals.faint_bg_color = GRUVBOX.dark0_soft;
    style.visuals.window_fill = BG;
    style.visuals.panel_fill = BG;
    style.visuals.window_stroke.color = FG;

    style.visuals.selection.bg_fill = GRUVBOX.dark4;
    style.visuals.selection.stroke.color = ACCENT;
    style.visuals.text_cursor.stroke.color = ACCENT;

    style.visuals.window_shadow.color = GRUVBOX.dark0_hard;
    style.visuals.popup_shadow.color = GRUVBOX.dark0_hard;

    style.visuals.slider_trailing_fill = true;
}

pub fn set_theme(theme: egui::Theme, style: &mut egui::style::Style) {
    match theme {
        egui::Theme::Dark => {
            set_dark_theme(style);
        }
        egui::Theme::Light => {
            set_light_theme(style);
        }
    }
}

pub struct NodeTheme {
    pub color: Color32,
    pub selected: Color32,
    pub hover: Color32,
    pub highlight: Color32,
}

pub struct GraphTheme {
    pub node: &'static NodeTheme,
    pub edge: Color32,
    pub out_link: Color32,
    pub backlink: Color32,
}

const NODE_DARK: NodeTheme = NodeTheme {
    color: GRUVBOX.neutral_purple,
    selected: GRUVBOX.bright_aqua,
    hover: GRUVBOX.bright_green,
    highlight: GRUVBOX.bright_blue,
};

const GRAPH_DARK: GraphTheme = GraphTheme {
    node: &NODE_DARK,
    edge: GRUVBOX.faded_green,
    out_link: GRUVBOX.bright_aqua,
    backlink: GRUVBOX.bright_purple,
};

const NODE_LIGHT: NodeTheme = NodeTheme {
    color: GRUVBOX.neutral_purple,
    selected: GRUVBOX.bright_aqua,
    hover: GRUVBOX.bright_green,
    highlight: GRUVBOX.bright_blue,
};

const GRAPH_LIGHT: GraphTheme = GraphTheme {
    node: &NODE_LIGHT,
    edge: GRUVBOX.faded_green,
    out_link: GRUVBOX.bright_aqua,
    backlink: GRUVBOX.bright_purple,
};

pub fn graph_style(theme: egui::Theme) -> &'static GraphTheme {
    match theme {
        egui::Theme::Dark => &GRAPH_DARK,
        egui::Theme::Light => &GRAPH_LIGHT,
    }
}
