//! ASCII art banner for interactive mode.

use std::io::IsTerminal;

/// ANSI true-color escape sequences for the banner palette.
struct Colors {
    shell_dark: &'static str,
    shell_light: &'static str,
    face: &'static str,
    eyes: &'static str,
    title: &'static str,
    subtitle: &'static str,
    reset: &'static str,
}

const COLOR: Colors = Colors {
    shell_dark: "\x1b[38;2;139;90;73m",
    shell_light: "\x1b[38;2;181;132;108m",
    face: "\x1b[38;2;194;150;130m",
    eyes: "\x1b[38;2;255;255;255m",
    title: "\x1b[1;38;2;220;165;120m",
    subtitle: "\x1b[38;2;100;100;120m",
    reset: "\x1b[0m",
};

const PLAIN: Colors = Colors {
    shell_dark: "",
    shell_light: "",
    face: "",
    eyes: "",
    title: "",
    subtitle: "",
    reset: "",
};

/// Prints the Edgelord banner to stdout.
///
/// Renders ANSI true-color when stdout is a terminal,
/// falls back to plain text otherwise.
pub fn print_banner() {
    let c = if std::io::stdout().is_terminal() {
        &COLOR
    } else {
        &PLAIN
    };

    let sd = c.shell_dark;
    let sl = c.shell_light;
    let fc = c.face;
    let ey = c.eyes;
    let tt = c.title;
    let st = c.subtitle;
    let r = c.reset;

    println!(
        r#"
{sd}     ▄▄▄▄▄▄▄▄▄{r}
{sl}   ▄█▒█▒█▒█▒█▒█▄{r}        {tt}    __________  ______________    ____  ____  ____{r}
{sd}  █▒█▒█▒█▒█▒█▒█▒█{r}       {tt}   / ____/ __ \/ ____/ ____/ /   / __ \/ __ \/ __ \{r}
{fc}  █▄▄▄▄▄▄▄▄▄▄▄▄▄█{r}       {tt}  / __/ / / / / / __/ __/ / /   / / / / /_/ / / / /{r}
{fc}  █░░░{ey}●{fc}░░░░░{ey}●{fc}░░░█{r}       {tt} / /___/ /_/ / /_/ / /___/ /___/ /_/ / _, _/ /_/ /{r}
{fc}  █░░░░░░░░░░░░░█{r}       {tt}/_____/_____/\____/_____/_____/\____/_/ |_/_____/{r}
{fc}   █░░░░▄▄░░░░░█{r}
{fc}    ▀█▄▄▄▄▄▄▄█▀{r}         {st}"This aggression will not stand, man."{r}
{fc}     ▀█▀   ▀█▀{r}
"#
    );
}
