pub mod document;
pub mod tag;

// use biome_rowan::TokenText;
// #[cfg(target_pointer_width = "64")]
// use biome_rowan::static_assert;
use std::hash::{Hash, Hasher};
use std::{borrow::Cow, ops::Deref, rc::Rc};

use super::{
    TagKind, TextSize, TokenText,
    format_element::tag::{LabelId, Tag},
};

/// Language agnostic IR for formatting source code.
///
/// Use the helper functions like [crate::builders::space], [crate::builders::soft_line_break] etc. defined in this file to create elements.
#[derive(Clone, Eq, PartialEq)]
pub enum FormatElement<'a> {
    /// A space token, see [crate::builders::space] for documentation.
    Space,
    HardSpace,
    /// A new line, see [crate::builders::soft_line_break], [crate::builders::hard_line_break], and [crate::builders::soft_line_break_or_space] for documentation.
    Line(LineMode),

    /// Forces the parent group to print in expanded mode.
    ExpandParent,

    /// Token constructed by the formatter from a static string
    StaticText {
        text: &'static str,
    },

    /// Token constructed from the input source as a dynamic
    /// string.
    DynamicText {
        text: &'a str,
    },

    /// A token for a text that is taken as is from the source code (input text and formatted representation are identical).
    /// Implementing by taking a slice from a `SyntaxToken` to avoid allocating a new string.
    LocatedTokenText {
        /// The start position of the token in the unformatted source code
        source_position: TextSize,
        /// The token text
        slice: TokenText,
    },

    /// Prevents that line suffixes move past this boundary. Forces the printer to print any pending
    /// line suffixes, potentially by inserting a hard line break.
    LineSuffixBoundary,

    /// An interned format element. Useful when the same content must be emitted multiple times to avoid
    /// deep cloning the IR when using the `best_fitting!` macro or `if_group_fits_on_line` and `if_group_breaks`.
    Interned(Interned<'a>),

    /// A list of different variants representing the same content. The printer picks the best fitting content.
    /// Line breaks inside of a best fitting don't propagate to parent groups.
    BestFitting(BestFittingElement<'a>),

    /// A [Tag] that marks the start/end of some content to which some special formatting is applied.
    Tag(Tag),
}

impl std::fmt::Debug for FormatElement<'_> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FormatElement::Space | FormatElement::HardSpace => fmt.write_str("Space"),
            FormatElement::Line(mode) => fmt.debug_tuple("Line").field(mode).finish(),
            FormatElement::ExpandParent => fmt.write_str("ExpandParent"),
            FormatElement::StaticText { text } => {
                fmt.debug_tuple("StaticText").field(text).finish()
            }
            FormatElement::DynamicText { text, .. } => {
                fmt.debug_tuple("DynamicText").field(text).finish()
            }
            FormatElement::LocatedTokenText { slice, .. } => {
                fmt.debug_tuple("LocatedTokenText").field(slice).finish()
            }
            FormatElement::LineSuffixBoundary => fmt.write_str("LineSuffixBoundary"),
            FormatElement::BestFitting(best_fitting) => {
                fmt.debug_tuple("BestFitting").field(&best_fitting).finish()
            }
            FormatElement::Interned(interned) => fmt.debug_list().entries(&**interned).finish(),
            FormatElement::Tag(tag) => fmt.debug_tuple("Tag").field(tag).finish(),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LineMode {
    /// See [crate::builders::soft_line_break_or_space] for documentation.
    SoftOrSpace,
    /// See [crate::builders::soft_line_break] for documentation.
    Soft,
    /// See [crate::builders::hard_line_break] for documentation.
    Hard,
    /// See [crate::builders::empty_line] for documentation.
    Empty,
}

impl LineMode {
    pub const fn is_hard(self) -> bool {
        matches!(self, LineMode::Hard)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PrintMode {
    /// Omits any soft line breaks
    Flat,
    /// Prints soft line breaks as line breaks
    Expanded,
}

impl PrintMode {
    pub const fn is_flat(self) -> bool {
        matches!(self, PrintMode::Flat)
    }

    pub const fn is_expanded(self) -> bool {
        matches!(self, PrintMode::Expanded)
    }
}

#[derive(Clone)]
pub struct Interned<'a>(Rc<[FormatElement<'a>]>);

impl<'a> Interned<'a> {
    pub(super) fn new(content: Vec<FormatElement<'a>>) -> Self {
        Self(content.into())
    }
}

impl PartialEq for Interned<'_> {
    fn eq(&self, other: &Interned<'_>) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Interned<'_> {}

impl Hash for Interned<'_> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        Rc::as_ptr(&self.0).hash(hasher);
    }
}

impl std::fmt::Debug for Interned<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a> Deref for Interned<'a> {
    type Target = [FormatElement<'a>];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

const LINE_SEPARATOR: char = '\u{2028}';
const PARAGRAPH_SEPARATOR: char = '\u{2029}';
pub const LINE_TERMINATORS: [char; 3] = ['\r', LINE_SEPARATOR, PARAGRAPH_SEPARATOR];

/// Replace the line terminators matching the provided list with "\n"
/// since its the only line break type supported by the printer
pub fn normalize_newlines<const N: usize>(text: &str, terminators: [char; N]) -> Cow<str> {
    let mut result = String::new();
    let mut last_end = 0;

    for (start, part) in text.match_indices(terminators) {
        result.push_str(&text[last_end..start]);
        result.push('\n');

        last_end = start + part.len();
        // If the current character is \r and the
        // next is \n, skip over the entire sequence
        if part == "\r" && text[last_end..].starts_with('\n') {
            last_end += 1;
        }
    }

    // If the result is empty no line terminators were matched,
    // return the entire input text without allocating a new String
    if result.is_empty() {
        Cow::Borrowed(text)
    } else {
        result.push_str(&text[last_end..text.len()]);
        Cow::Owned(result)
    }
}

impl FormatElement<'_> {
    /// Returns `true` if self is a [FormatElement::Tag]
    pub const fn is_tag(&self) -> bool {
        matches!(self, FormatElement::Tag(_))
    }

    /// Returns `true` if self is a [FormatElement::Tag] and [Tag::is_start] is `true`.
    pub const fn is_start_tag(&self) -> bool {
        match self {
            FormatElement::Tag(tag) => tag.is_start(),
            _ => false,
        }
    }

    /// Returns `true` if self is a [FormatElement::Tag] and [Tag::is_end] is `true`.
    pub const fn is_end_tag(&self) -> bool {
        match self {
            FormatElement::Tag(tag) => tag.is_end(),
            _ => false,
        }
    }

    pub const fn is_text(&self) -> bool {
        matches!(
            self,
            FormatElement::LocatedTokenText { .. }
                | FormatElement::DynamicText { .. }
                | FormatElement::StaticText { .. }
        )
    }

    pub const fn is_space(&self) -> bool {
        matches!(self, FormatElement::Space)
    }

    pub const fn is_line(&self) -> bool {
        matches!(self, FormatElement::Line(_))
    }
}

impl FormatElements for FormatElement<'_> {
    fn will_break(&self) -> bool {
        match self {
            FormatElement::ExpandParent => true,
            FormatElement::Tag(Tag::StartGroup(group)) => !group.mode().is_flat(),
            FormatElement::Line(line_mode) => matches!(line_mode, LineMode::Hard | LineMode::Empty),
            FormatElement::StaticText { text } | FormatElement::DynamicText { text } => {
                text.contains('\n')
            }
            FormatElement::LocatedTokenText { slice, .. } => slice.contains('\n'),
            FormatElement::Interned(interned) => interned.will_break(),
            // Traverse into the most flat version because the content is guaranteed to expand when even
            // the most flat version contains some content that forces a break.
            FormatElement::BestFitting(best_fitting) => best_fitting.most_flat().will_break(),
            FormatElement::LineSuffixBoundary
            | FormatElement::Space
            | FormatElement::Tag(_)
            | FormatElement::HardSpace => false,
        }
    }

    fn may_directly_break(&self) -> bool {
        matches!(self, FormatElement::Line(_))
    }

    fn has_label(&self, label_id: LabelId) -> bool {
        match self {
            FormatElement::Tag(Tag::StartLabelled(actual)) => *actual == label_id,
            FormatElement::Interned(interned) => interned.deref().has_label(label_id),
            _ => false,
        }
    }

    fn start_tag(&self, _: TagKind) -> Option<&Tag> {
        None
    }

    fn end_tag(&self, kind: TagKind) -> Option<&Tag> {
        match self {
            FormatElement::Tag(tag) if tag.kind() == kind && tag.is_end() => Some(tag),
            _ => None,
        }
    }
}

/// Provides the printer with different representations for the same element so that the printer
/// can pick the best fitting variant.
///
/// Best fitting is defined as the variant that takes the most horizontal space but fits on the line.
#[derive(Clone, Eq, PartialEq)]
pub struct BestFittingElement<'a> {
    /// The different variants for this element.
    /// The first element is the one that takes up the most space horizontally (the most flat),
    /// The last element takes up the least space horizontally (but most horizontal space).
    variants: Box<[Box<[FormatElement<'a>]>]>,
}

impl<'a> BestFittingElement<'a> {
    /// Creates a new best fitting IR with the given variants. The method itself isn't unsafe
    /// but it is to discourage people from using it because the printer will panic if
    /// the slice doesn't contain at least the least and most expanded variants.
    ///
    /// You're looking for a way to create a `BestFitting` object, use the `best_fitting![least_expanded, most_expanded]` macro.
    ///
    /// ## Safety
    /// The slice must contain at least two variants.
    #[doc(hidden)]
    pub unsafe fn from_vec_unchecked(variants: Vec<Box<[FormatElement<'a>]>>) -> Self {
        debug_assert!(
            variants.len() >= 2,
            "Requires at least the least expanded and most expanded variants"
        );

        Self { variants: variants.into_boxed_slice() }
    }

    /// Returns the most expanded variant
    pub fn most_expanded(&self) -> &[FormatElement<'a>] {
        self.variants.last().expect(
            "Most contain at least two elements, as guaranteed by the best fitting builder.",
        )
    }

    pub fn variants(&self) -> &[Box<[FormatElement<'a>]>] {
        &self.variants
    }

    /// Returns the least expanded variant
    pub fn most_flat(&self) -> &[FormatElement<'a>] {
        self.variants.first().expect(
            "Most contain at least two elements, as guaranteed by the best fitting builder.",
        )
    }
}

impl std::fmt::Debug for BestFittingElement<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(&*self.variants).finish()
    }
}

pub trait FormatElements {
    /// Returns true if this [FormatElement] is guaranteed to break across multiple lines by the printer.
    /// This is the case if this format element recursively contains a:
    /// * [crate::builders::empty_line] or [crate::builders::hard_line_break]
    /// * A token containing '\n'
    ///
    /// Use this with caution, this is only a heuristic and the printer may print the element over multiple
    /// lines if this element is part of a group and the group doesn't fit on a single line.
    fn will_break(&self) -> bool;

    /// Returns true if this [FormatElement] has the potential to break across multiple lines when printed.
    /// This is the case _only_ if this format element recursively contains a [FormatElement::Line].
    ///
    /// It's possible for [FormatElements::will_break] to return true while this function returns false,
    /// such as when the group contains a [crate::builders::expand_parent] or some text within the group
    /// contains a newline. Neither of those cases directly contain a [FormatElement::Line], and so they
    /// do not _directly_ break.
    fn may_directly_break(&self) -> bool;

    /// Returns true if the element has the given label.
    fn has_label(&self, label: LabelId) -> bool;

    /// Returns the start tag of `kind` if:
    /// * the last element is an end tag of `kind`.
    /// * there's a matching start tag in this document (may not be true if this slice is an interned element and the `start` is in the document storing the interned element).
    fn start_tag(&self, kind: TagKind) -> Option<&Tag>;

    /// Returns the end tag if:
    /// * the last element is an end tag of `kind`
    fn end_tag(&self, kind: TagKind) -> Option<&Tag>;
}
