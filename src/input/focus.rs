use std::cell::RefCell;

use crate::Key;

use super::KeyHandler;

// the text input that typed keys go to, and what it does with them
struct Focus {
    id: &'static str,
    on_key: KeyHandler,
}

thread_local! {
    // only one text input takes keys at a time
    static FOCUS: RefCell<Option<Focus>> = const { RefCell::new(None) };
}

pub fn set(id: &'static str, on_key: KeyHandler) {
    FOCUS.set(Some(Focus { id, on_key }));
}

pub fn clear() {
    FOCUS.set(None);
}

pub fn has(id: &'static str) -> bool {
    FOCUS.with_borrow(|focus| matches!(focus, Some(focus) if focus.id == id))
}

// returns whether a text input took the key
pub fn send(key: Key) -> bool {
    // a text input has no use for these, so they go on to the window's on_key
    if matches!(key, Key::Up | Key::Down | Key::Tab | Key::Other) {
        return false;
    }

    // copied out first, so the handler is free to change the focus itself
    let on_key = FOCUS.with_borrow(|focus| focus.as_ref().map(|focus| focus.on_key.clone()));

    let Some(on_key) = on_key else {
        return false;
    };

    on_key(key);

    /*
     * escape leaves the input and still reaches the window, which may want to close.
     * left and right move the input's cursor and reach the window too, which may have
     * something of its own to move along, like a grid under a search box
     */
    !matches!(key, Key::Escape | Key::Left | Key::Right)
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;

    // every key the input was handed, in the order it was handed them
    fn recording() -> Rc<RefCell<Vec<Key>>> {
        let keys = Rc::new(RefCell::new(Vec::new()));
        let seen = keys.clone();

        set(
            "input",
            Rc::new(move |key| seen.borrow_mut().push(key)),
        );

        keys
    }

    #[test]
    fn a_window_hears_the_keys_an_input_leaves_to_it() {
        let keys = recording();

        // typing is the input's, and the window never sees it
        assert!(send(Key::Character('a')));
        assert!(send(Key::Backspace));

        // moving along the text is the window's to hear as well
        assert!(!send(Key::Left));
        assert!(!send(Key::Right));

        // walking a list skips the input altogether, so its handler never sees these
        assert!(!send(Key::Up));
        assert!(!send(Key::Down));
        assert!(!send(Key::Tab));

        // escape takes the focus away and reaches the window
        assert!(!send(Key::Escape));

        // an input that never saw them hands every key on
        clear();

        assert!(!send(Key::Character('a')));
        assert!(!send(Key::Left));

        // the input was handed its own keys, and escape with them; up, down and tab
        // skipped it, and nothing was handed it once the focus was gone
        assert_eq!(
            *keys.borrow(),
            vec![
                Key::Character('a'),
                Key::Backspace,
                Key::Left,
                Key::Right,
                Key::Escape,
            ]
        );
    }
}
