use gtk::prelude::*;
use itertools::Itertools;

pub fn toggle_line_start(buf: &gtk::TextBuffer, starts: &[&str]) {
    let mut to_insert: &str = starts.get(0).unwrap();
    let mut clear_chars = 0;
    let mut iter = buf.iter_at_offset(buf.cursor_position());
    iter.backward_chars(iter.line_offset());
    let mut iter2 = buf.start_iter();
    for (i, header) in starts.iter().enumerate() {
        iter2.set_offset(iter.offset() + header.len() as i32);
        if buf.text(&iter, &iter2, false).to_string().as_str() == *header {
            // this pattern is in use, next time
            // we want to move to the next pattern
            to_insert = if i + 1 >= starts.len() {
                ""
            } else {
                starts[i + 1]
            };
            clear_chars = header.len() as i32;
            break;
        }
    }
    if clear_chars > 0 {
        iter2.set_offset(iter.offset() + clear_chars);
        buf.delete(&mut iter, &mut iter2);
    }
    buf.insert(&mut iter, to_insert);
}

pub fn toggle_snippet(buf: &gtk::TextBuffer, before: &str, after: &str) {
    let before_len = before.len() as i32;
    let after_len = after.len() as i32;
    let (start_offset, end_offset) = match buf.selection_bounds() {
        None => {
            // no selection
            let cursor_iter = buf.iter_at_mark(&buf.get_insert());
            let offset = cursor_iter.offset();
            (offset, offset)
        }
        Some((sel_start_iter, sel_end_iter)) => {
            // selection
            (sel_start_iter.offset(), sel_end_iter.offset())
        }
    };
    let mut iter = buf.iter_at_offset(end_offset);

    // if the selection is [**test**] and the user clicked bold, should we
    // un-toggle, meaning change the contents to [test]?
    let is_untoggle = start_offset >= before_len && {
        let mut iter2 = buf.iter_at_offset(end_offset + after_len);
        if buf.text(&iter2, &iter, false) != after {
            false
        } else {
            let iter1 = buf.iter_at_offset(start_offset);
            iter2.set_offset(start_offset - before_len);
            buf.text(&iter1, &iter2, false) == before
        }
    };

    if is_untoggle {
        // untoggle => remove the 'before' and 'after' strings
        let mut iter2 = buf.iter_at_offset(end_offset + after_len);
        buf.delete(&mut iter, &mut iter2);
        iter.set_offset(start_offset - before_len);
        iter2.set_offset(start_offset);
        buf.delete(&mut iter, &mut iter2);
        // restore the selection
        iter.set_offset(start_offset - before_len);
        iter2.set_offset(end_offset - before_len);
        buf.select_range(&iter, &iter2);
    } else {
        // plain toggle, add the 'before' and 'after' strings
        buf.insert(&mut iter, after);
        iter.set_offset(start_offset);
        buf.insert(&mut iter, before);
        iter.set_offset(start_offset);
        iter.set_offset(start_offset + before_len);
        if start_offset < end_offset {
            // restore the selection
            let iter_end = buf.iter_at_offset(end_offset + before_len);
            buf.select_range(&iter, &iter_end);
        } else {
            buf.place_cursor(&iter);
        }
    }
}

pub fn toggle_password(buf: &gtk::TextBuffer) {
    let sel_bounds = buf.selection_bounds();
    if sel_bounds.is_none() {
        // no selection
        toggle_snippet(buf, "[pass`", "`]");
        return;
    }
    let (start_iter, end_iter) = sel_bounds.unwrap();
    let selected_text = buf.text(&start_iter, &end_iter, false).to_string();
    let mut separator = "`".to_string();
    while selected_text.contains(&separator) {
        separator.push('`');
    }
    let extra_space = if selected_text.starts_with('`') || selected_text.ends_with('`') {
        " "
    } else {
        ""
    };
    let before = "[pass".to_string() + &separator + extra_space;
    let after = extra_space.to_string() + &separator + "]";
    toggle_snippet(buf, &before, &after);
}

pub fn toggle_preformat(buf: &gtk::TextBuffer) {
    let sel_bounds = buf.selection_bounds();
    if sel_bounds.is_none() {
        // no selection
        toggle_snippet(buf, "`", "`");
        return;
    }
    let (start_iter, end_iter) = sel_bounds.unwrap();
    let selected_text = buf.text(&start_iter, &end_iter, false).to_string();
    if selected_text.contains('\n') {
        // multiline
        toggle_snippet(buf, "\n```\n", "\n```\n");
    } else {
        // single line
        toggle_snippet(buf, "`", "`");
    }
}

pub fn toggle_blockquote(buf: &gtk::TextBuffer) {
    let (start_offset, end_offset) = match buf.selection_bounds() {
        None => {
            // no selection
            let cursor_iter = buf.iter_at_mark(&buf.get_insert());
            let offset = cursor_iter.offset();
            (offset, offset)
        }
        Some((sel_start_iter, sel_end_iter)) => {
            // selection
            (sel_start_iter.offset(), sel_end_iter.offset())
        }
    };
    let mut iter = buf.iter_at_offset(end_offset);
    if start_offset != end_offset {
        // there is a selection
        let mut start_iter = buf.iter_at_offset(start_offset);
        let selected_text = buf.text(&start_iter, &iter, false).to_string();
        let lines: Vec<_> = selected_text.lines().collect();
        let next_selection: String = if lines.iter().all(|l| l.starts_with("> ")) {
            // remove the blockquote
            Itertools::intersperse(lines.iter().map(|l| &l[2..]), "\n").collect()
        } else {
            // add the blockquote
            Itertools::intersperse(lines.iter().map(|l| format!("> {}", l)), "\n".to_string())
                .collect()
        };
        buf.delete(&mut start_iter, &mut iter);
        start_iter.set_offset(start_offset);
        buf.insert(&mut start_iter, &next_selection);
        // for the apidoc of textbuffer::insert:
        // iter is invalidated when insertion occurs, but the default signal handler
        // revalidates it to point to the end of the inserted text.
        // => start_iter now points to the end of the inserted text
        // iter.set_offset(start_offset); <-- for some reason iter is invalidated & even set_offset can't recover it
        buf.select_range(&buf.iter_at_offset(start_offset), &start_iter);
    } else {
        // no selection
        iter.backward_chars(iter.line_offset());
        let mut iter2 = buf.iter_at_offset(iter.offset() + 2);
        if buf.text(&iter, &iter2, false).to_string().as_str() == "> " {
            buf.delete(&mut iter, &mut iter2);
        } else {
            buf.insert(&mut iter, "> ");
        }
    }
}
