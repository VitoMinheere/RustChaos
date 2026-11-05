//! `data\urban.sty`, the game's mission list: one line per mission,
//! `id : group : parent : ... : district : file.ucm : title : briefing`, fields separated by
//! `" : "`; `//` comments; then a `[districts]` list.

use std::path::Path;

/// The install's `urban.sty` as text (a byte per character), empty if it has none.
pub fn read(game: &Path) -> String {
    crate::resolve(game, "data\\urban.sty")
        .and_then(|p| std::fs::read(p).ok())
        .map(|b| b.iter().map(|&c| c as char).collect())
        .unwrap_or_default()
}

/// The lines of `text` that aren't comments, as fields. Any `[districts]` lines come too
/// (as one field each); see [`Line::parse`].
pub fn lines(text: &str) -> impl Iterator<Item = Line<'_>> {
    text.lines().filter_map(Line::parse)
}

/// A line of `urban.sty`, split into its fields (each trimmed).
pub struct Line<'a> {
    fields: Vec<&'a str>,
}

impl<'a> Line<'a> {
    /// `line`'s fields; none if it's a `//` comment.
    pub fn parse(line: &'a str) -> Option<Line<'a>> {
        (!line.trim_start().starts_with("//")).then(|| Line { fields: line.split(" : ").map(str::trim).collect() })
    }

    /// The mission's id, the first field.
    pub fn id(&self) -> Option<u32> {
        self.fields.first()?.parse().ok()
    }

    /// The district it's in on the city map, the seventh field.
    pub fn district(&self) -> Option<usize> {
        self.fields.get(6)?.parse().ok()
    }

    /// The mission file without its `.ucm` (as it's written), the eighth field; none if the
    /// field isn't a `.ucm` file.
    pub fn mission(&self) -> Option<&'a str> {
        let file = *self.fields.get(7)?;
        file.to_ascii_lowercase().ends_with(".ucm").then(|| &file[..file.len() - 4])
    }

    /// The mission's title, the ninth field.
    pub fn title(&self) -> Option<&'a str> {
        self.fields.get(8).copied()
    }

    /// The mission's briefing, the tenth field.
    pub fn briefing(&self) -> Option<&'a str> {
        self.fields.get(9).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields() {
        let l = Line::parse("3 : 1 : 0 : 0 : 0 : 0 : 2 : Gang1.UCM : Gang Order : Go there.").unwrap();
        assert_eq!(l.id(), Some(3));
        assert_eq!(l.district(), Some(2));
        assert_eq!(l.mission(), Some("Gang1"));
        assert_eq!(l.title(), Some("Gang Order"));
        assert_eq!(l.briefing(), Some("Go there."));
        assert!(Line::parse("  // a comment").is_none());
        let short = Line::parse("1 : 2 : 3 : 4 : 5 : 6 : 7 : notes.txt").unwrap();
        assert_eq!((short.mission(), short.title()), (None, None));
    }
}
