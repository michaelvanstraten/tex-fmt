use crate::comments::*;
use crate::ignore::*;
use crate::leave::*;
use crate::logging::*;
use crate::parse::*;
use crate::regexes::*;
use crate::TAB;
use core::cmp::max;
use log::Level::{Info, Trace, Warn};

const OPENS: [char; 3] = ['(', '[', '{'];
const CLOSES: [char; 3] = [')', ']', '}'];

#[derive(Debug, Clone)]
pub struct Indent {
    pub actual: i8,
    pub visual: i8,
}

impl Indent {
    fn new() -> Self {
        Indent {
            actual: 0,
            visual: 0,
        }
    }
}

// calculate total indentation change due to current line
fn get_diff(line: &str) -> i8 {
    // documents get no global indentation
    if RE_DOCUMENT_BEGIN.is_match(line) || RE_DOCUMENT_END.is_match(line) {
        return 0;
    };

    // list environments get double indents
    let mut diff: i8 = 0;

    // other environments get single indents
    if RE_ENV_BEGIN.is_match(line) {
        diff += 1;
        for re_list_begin in RE_LISTS_BEGIN.iter() {
            if re_list_begin.is_match(line) {
                diff += 1
            };
        }
    } else if RE_ENV_END.is_match(line) {
        diff -= 1;
        for re_list_end in RE_LISTS_END.iter() {
            if re_list_end.is_match(line) {
                diff -= 1
            };
        }
    };

    // indent for delimiters
    diff += line.chars().filter(|x| OPENS.contains(x)).count() as i8;
    diff -= line.chars().filter(|x| CLOSES.contains(x)).count() as i8;

    diff
}

// calculate dedentation for current line compared to previous
fn get_back(line: &str) -> i8 {
    // documents get no global indentation
    if RE_DOCUMENT_END.is_match(line) {
        return 0;
    };

    let mut back: i8 = 0;
    let mut cumul: i8 = 0;

    // delimiters
    for c in line.chars() {
        cumul -= OPENS.contains(&c) as i8;
        cumul += CLOSES.contains(&c) as i8;
        back = max(cumul, back);
    }

    // other environments get single indents
    if RE_ENV_END.is_match(line) {
        // list environments get double indents for indenting items
        for re_list_end in RE_LISTS_END.iter() {
            if re_list_end.is_match(line) {
                return 2;
            };
        }
        back += 1;
    };

    // deindent items to make the rest of item environment appear indented
    if RE_ITEM.is_match(line) {
        back += 1;
    };

    back
}

fn get_indent(line: &str, prev_indent: Indent) -> Indent {
    let diff = get_diff(line);
    let back = get_back(line);
    let actual = prev_indent.actual + diff;
    let visual = prev_indent.actual - back;
    Indent { actual, visual }
}

pub fn apply_indent(
    file: &str,
    filename: &str,
    args: &Cli,
    logs: &mut Vec<Log>,
    pass: Option<usize>,
) -> String {
    if args.verbose {
        record_log(
            logs,
            Info,
            pass,
            filename.to_string(),
            None,
            None,
            format!("Indent on pass {}.", pass.unwrap_or_default()),
        );
    }

    let mut indent = Indent::new();
    let mut ignore = Ignore::new();
    let mut leave = Leave::new();
    let mut new_file = String::with_capacity(file.len());

    for (linum, line) in file.lines().enumerate() {
        ignore = get_ignore(line, linum, ignore, filename, logs, pass, true);
        leave = get_leave(line, linum, leave, filename, logs, pass, true);

        if !leave.visual && !ignore.visual {
            // calculate indent
            let comment_index = find_comment_index(line);
            let line_strip = remove_comment(line, comment_index);
            indent = get_indent(line_strip, indent);
            if args.trace {
                record_log(
                    logs,
                    Trace,
                    pass,
                    filename.to_string(),
                    Some(linum),
                    Some(line.to_string()),
                    format!(
                        "Indent: actual = {}, visual = {}:",
                        indent.actual, indent.visual
                    ),
                );
            }

            if (indent.visual < 0) || (indent.actual < 0) {
                record_log(
                    logs,
                    Warn,
                    pass,
                    filename.to_string(),
                    Some(linum),
                    Some(line.to_string()),
                    "Indent is negative.".to_string(),
                );
                indent.actual = indent.actual.max(0);
                indent.visual = indent.visual.max(0);
            }

            // apply indent
            let mut new_line = line.trim_start().to_string();
            if !new_line.is_empty() {
                let n_spaces = indent.visual * TAB;
                for _ in 0..n_spaces {
                    new_line.insert(0, ' ');
                }
            }
            new_file.push_str(&new_line);
        } else {
            new_file.push_str(line);
        }
        new_file.push('\n');
    }

    new_file
}
