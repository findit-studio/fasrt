#[cfg(any(feature = "alloc", feature = "std"))]
use derive_more::{TryUnwrap, Unwrap};

use super::*;

#[cfg(any(feature = "alloc", feature = "std"))]
use crate::error::MaxDepthExceededError;

#[cfg(any(feature = "alloc", feature = "std"))]
use crate::vtt::Timestamp;

#[cfg(any(feature = "alloc", feature = "std"))]
use std::vec::Vec;

#[cfg(any(feature = "alloc", feature = "std"))]
mod sealed {
  use super::*;

  pub trait Sealed {}

  impl Sealed for Vec<super::Node<'_>> {}
  impl Sealed for &[super::Node<'_>] {}
  impl<const N: usize> Sealed for [super::Node<'_>; N] {}
  impl Sealed for super::Node<'_> {}
}

/// Trait for types that can serve as a container of [`Node`]s.
///
/// Sealed — implemented for [`Vec<Node>`], `&[Node]`, and `[Node; N]`.
#[cfg(any(feature = "alloc", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "alloc", feature = "std"))))]
pub trait Nodes<'a>: sealed::Sealed {
  /// View the contained nodes as a slice.
  fn as_nodes(&self) -> &[Node<'a>];
}

/// A node in the cue text DOM tree.
#[derive(Debug, Clone, PartialEq, Eq, IsVariant, Unwrap, TryUnwrap)]
#[unwrap(ref, ref_mut)]
#[try_unwrap(ref, ref_mut)]
#[cfg(any(feature = "alloc", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "alloc", feature = "std"))))]
pub enum Node<'a> {
  /// A text node.
  Text(CueStr<'a>),
  /// A timestamp node.
  Timestamp(Timestamp),
  /// A tag node with children.
  Tag(TagNode<'a>),
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "alloc", feature = "std"))))]
const _: () = {
  impl<'a> Nodes<'a> for Vec<Node<'a>> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn as_nodes(&self) -> &[Node<'a>] {
      self.as_slice()
    }
  }

  impl<'a> Nodes<'a> for &'a [Node<'a>] {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn as_nodes(&self) -> &[Node<'a>] {
      self
    }
  }

  impl<'a, const N: usize> Nodes<'a> for [Node<'a>; N] {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn as_nodes(&self) -> &[Node<'a>] {
      self.as_slice()
    }
  }

  impl<'a> Nodes<'a> for Node<'a> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn as_nodes(&self) -> &[Node<'a>] {
      core::slice::from_ref(self)
    }
  }

  impl<'a> AsRef<[Node<'a>]> for Node<'a> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn as_ref(&self) -> &[Node<'a>] {
      core::slice::from_ref(self)
    }
  }

  impl<'a> AsMut<[Node<'a>]> for Node<'a> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn as_mut(&mut self) -> &mut [Node<'a>] {
      core::slice::from_mut(self)
    }
  }

  impl AsRef<Self> for Node<'_> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn as_ref(&self) -> &Self {
      self
    }
  }

  impl AsMut<Self> for Node<'_> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn as_mut(&mut self) -> &mut Self {
      self
    }
  }

  impl fmt::Display for Node<'_> {
    /// Serializes the node to WebVTT cue text markup.
    ///
    /// ```rust
    /// # #[cfg(any(feature = "alloc", feature = "std"))]
    /// # {
    /// use fasrt::vtt::cue::{Node, CueStr};
    ///
    /// let node = Node::Text(CueStr::borrowed("hello"));
    /// assert_eq!(node.to_string(), "hello");
    /// # }
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      match self {
        Node::Text(s) => fmt::Display::fmt(s, f),
        Node::Timestamp(ts) => write!(f, "<{}>", ts.encode().as_str()),
        Node::Tag(tag) => fmt::Display::fmt(tag, f),
      }
    }
  }
};

/// A tag node in the cue text DOM tree, generic over its children
/// container.
///
/// The default container is `Vec<Node<'a>>`, used by the parser.  For
/// allocation-free writing you can use `[Node; N]` or `&[Node]` instead.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(any(feature = "alloc", feature = "std"))]
pub struct TagNode<'a, C = Vec<Node<'a>>> {
  tag: Tag,
  classes: &'a str,
  annotation: Option<Annotation<'a>>,
  children: C,
}

/// A tag node in the cue text DOM tree, generic over its children
/// container.
///
/// The default container is `Vec<Node<'a>>`, used by the parser.  For
/// allocation-free writing you can use `[Node; N]` or `&[Node]` instead.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(not(any(feature = "alloc", feature = "std")))]
pub struct TagNode<'a, C> {
  tag: Tag,
  classes: &'a str,
  annotation: Option<Annotation<'a>>,
  children: C,
}

/// A lazy iterator over a start tag's classes, as W3C WebVTT §6.4 reads them.
///
/// Created by [`TagNode::classes`], or from a [`CueToken::StartTag`]'s raw
/// class list with [`Classes::new`].
///
/// The raw list is the dot-separated text between a tag name and its
/// annotation. §6.4 attaches a node whose "list of applicable classes" is that
/// text split on U+002E FULL STOP, *"excluding any classes that are the empty
/// string"* — so a run of dots contributes nothing: `<c.a..b>` has the two
/// classes `a` and `b`, and `<c.>` has none.
///
/// ```rust
/// use fasrt::vtt::cue::Classes;
///
/// let classes: Vec<_> = Classes::new("a..b").collect();
/// assert_eq!(classes, ["a", "b"]);
/// assert_eq!(Classes::new("..").next(), None);
/// ```
///
/// [`CueToken::StartTag`]: crate::vtt::cue::CueToken::StartTag
#[derive(Debug, Clone)]
pub struct Classes<'a> {
  rest: &'a str,
}

impl<'a> Classes<'a> {
  /// Create a class iterator over a start tag's raw dot-separated class list.
  ///
  /// ```rust
  /// use fasrt::vtt::cue::{Classes, CueParser, CueToken};
  ///
  /// let mut parser = CueParser::new("<c.loud.important>text</c>");
  /// let Some(CueToken::StartTag { classes, .. }) = parser.next() else {
  ///   unreachable!()
  /// };
  /// assert_eq!(Classes::new(classes).collect::<Vec<_>>(), ["loud", "important"]);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(raw: &'a str) -> Self {
    Self { rest: raw }
  }
}

impl<'a> Iterator for Classes<'a> {
  type Item = &'a str;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      if self.rest.is_empty() {
        return None;
      }
      match self.rest.find('.') {
        // U+002E is one byte, so `idx + 1` is a char boundary.
        Some(idx) => {
          let (class, rest) = self.rest.split_at(idx);
          self.rest = &rest[1..];
          if !class.is_empty() {
            return Some(class);
          }
        }
        // The tail holds no separator, and it is not empty: it is the last
        // class, and the iterator is done after it.
        None => return Some(core::mem::take(&mut self.rest)),
      }
    }
  }
}

impl core::iter::FusedIterator for Classes<'_> {}

impl<'a, C> AsRef<[TagNode<'a, C>]> for TagNode<'a, C> {
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn as_ref(&self) -> &[TagNode<'a, C>] {
    core::slice::from_ref(self)
  }
}

impl<'a, C> AsMut<[TagNode<'a, C>]> for TagNode<'a, C> {
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn as_mut(&mut self) -> &mut [TagNode<'a, C>] {
    core::slice::from_mut(self)
  }
}

impl<C> AsRef<Self> for TagNode<'_, C> {
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn as_ref(&self) -> &Self {
    self
  }
}

impl<C> AsMut<Self> for TagNode<'_, C> {
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn as_mut(&mut self) -> &mut Self {
    self
  }
}

// Methods available on all TagNode<C> variants.
impl<'a, C> TagNode<'a, C> {
  /// Returns the tag name.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let node = TagNode::new(Tag::Italic);
  /// assert_eq!(node.tag(), Tag::Italic);
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn tag(&self) -> Tag {
    self.tag
  }

  /// Sets the tag name (builder pattern).
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let node = TagNode::new(Tag::Bold).with_tag(Tag::Italic);
  /// assert_eq!(node.tag(), Tag::Italic);
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn with_tag(mut self, tag: Tag) -> Self {
    self.tag = tag;
    self
  }

  /// Sets the tag name.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let mut node = TagNode::new(Tag::Bold);
  /// node.set_tag(Tag::Underline);
  /// assert_eq!(node.tag(), Tag::Underline);
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn set_tag(&mut self, tag: Tag) -> &mut Self {
    self.tag = tag;
    self
  }

  /// Returns the node's classes, as W3C WebVTT §6.4 reads them: the raw list
  /// split on U+002E FULL STOP, *"excluding any classes that are the empty
  /// string"*.
  ///
  /// Use [`classes_raw`](Self::classes_raw) for the undivided source text.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let node = TagNode::new(Tag::Class).with_classes("loud.important");
  /// assert_eq!(node.classes().collect::<Vec<_>>(), ["loud", "important"]);
  ///
  /// // A run of dots yields no empty class.
  /// let node = TagNode::new(Tag::Class).with_classes("a..b");
  /// assert_eq!(node.classes().collect::<Vec<_>>(), ["a", "b"]);
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn classes(&self) -> Classes<'a> {
    Classes::new(self.classes)
  }

  /// Returns the raw dot-separated class list, exactly as it appeared
  /// (e.g. `"loud.important"`), empty if the tag declared none.
  ///
  /// This is the source text, not §6.4's class list: it still carries any
  /// empty classes the spec excludes. Use [`classes`](Self::classes) to read
  /// the list itself.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let node = TagNode::new(Tag::Class).with_classes("a..b");
  /// assert_eq!(node.classes_raw(), "a..b");
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn classes_raw(&self) -> &'a str {
    self.classes
  }

  /// Sets the raw dot-separated class list (builder pattern).
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let node = TagNode::new(Tag::Class).with_classes("highlight");
  /// assert_eq!(node.classes_raw(), "highlight");
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn with_classes(mut self, classes: &'a str) -> Self {
    self.classes = classes;
    self
  }

  /// Sets the raw dot-separated class list.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let mut node = TagNode::new(Tag::Class);
  /// node.set_classes("loud");
  /// assert_eq!(node.classes_raw(), "loud");
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn set_classes(&mut self, classes: &'a str) -> &mut Self {
    self.classes = classes;
    self
  }

  /// Returns the node's annotation — `<v>`'s voice, `<lang>`'s language —
  /// `None` if the tag declared none.
  ///
  /// [`Annotation::normalize`] gives W3C WebVTT §6.4's value for it, with the
  /// character references decoded and the whitespace runs collapsed;
  /// [`Annotation::as_raw`] gives the text the cue spelled, which is what the
  /// node is serialized from. For a `<lang>`, [`declared_language`] answers the
  /// same question in the shape §6.4's language stack asks it.
  ///
  /// [`declared_language`]: Self::declared_language
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{Annotation, TagNode, Tag};
  ///
  /// let node = TagNode::new(Tag::Voice).with_annotation(Some(Annotation::new("Speaker")));
  /// assert_eq!(node.annotation().map(Annotation::normalize), Some("Speaker"));
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn annotation(&self) -> Option<&Annotation<'a>> {
    self.annotation.as_ref()
  }

  /// Sets the annotation (builder pattern).
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{Annotation, TagNode, Tag};
  ///
  /// let node = TagNode::new(Tag::Lang).with_annotation(Some(Annotation::new("en")));
  /// assert_eq!(node.annotation().map(Annotation::as_raw), Some("en"));
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn with_annotation(mut self, annotation: Option<Annotation<'a>>) -> Self {
    self.annotation = annotation;
    self
  }

  /// Sets the annotation.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{Annotation, TagNode, Tag};
  ///
  /// let mut node = TagNode::new(Tag::Voice);
  /// node.set_annotation(Some(Annotation::new("Roger")));
  /// assert_eq!(node.annotation().map(Annotation::as_raw), Some("Roger"));
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn set_annotation(&mut self, annotation: Option<Annotation<'a>>) -> &mut Self {
    self.annotation = annotation;
    self
  }

  /// Returns the language this node makes applicable to itself and its
  /// descendants, or `None` when it leaves the enclosing language standing.
  ///
  /// This is §6.4's language stack, read off the tree: `<lang>` is the only
  /// tag that pushes, and it pushes its annotation. An annotation-less
  /// `<lang>` pushes the empty string, which is a language entry like any
  /// other — it *clears* an enclosing language rather than inheriting it, so
  /// this returns `Some("")` and not `None`.
  ///
  /// [`CueText::nodes_with_language`] walks a whole tree with this rule
  /// applied; use this accessor when descending the tree yourself.
  ///
  /// # The value is §6.4's annotation
  ///
  /// What §6.4 pushes is the annotation its annotation state produced, so what
  /// is returned is [`Annotation::normalize`]: `<lang en&#x2D;US>` declares
  /// `"en-US"`, and `<lang en` + TAB + `US>` declares `"en US"`. Read
  /// [`annotation`](Self::annotation) for the text the cue spelled.
  ///
  /// Without `alloc` a decoded language has nowhere to live, so the stored text
  /// is returned instead; [`Annotation::requires_normalization`] on the
  /// annotation says when the two differ.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{Annotation, TagNode, Tag};
  ///
  /// let lang = TagNode::new(Tag::Lang).with_annotation(Some(Annotation::new("en")));
  /// assert_eq!(lang.declared_language(), Some("en"));
  ///
  /// // `<lang>` with no annotation clears the enclosing language.
  /// assert_eq!(TagNode::new(Tag::Lang).declared_language(), Some(""));
  ///
  /// // Every other tag declares nothing — `<v>`'s annotation is a voice.
  /// let voice = TagNode::new(Tag::Voice).with_annotation(Some(Annotation::new("Esme")));
  /// assert_eq!(voice.declared_language(), None);
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn declared_language(&self) -> Option<&str> {
    match self.tag {
      Tag::Lang => Some(match &self.annotation {
        Some(language) => language.normalize(),
        None => "",
      }),
      _ => None,
    }
  }

  /// Returns the child nodes as a slice.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let node = TagNode::new(Tag::Bold);
  /// assert!(node.children().is_empty());
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn children(&self) -> &C {
    &self.children
  }

  /// Returns the child nodes as a slice.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let mut node = TagNode::new(Tag::Bold);
  /// assert!(node.children_mut().is_empty());
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn children_mut(&mut self) -> &mut C {
    &mut self.children
  }

  /// Replaces the children container, potentially changing the container
  /// type.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Node, CueStr, Tag};
  ///
  /// let node = TagNode::new(Tag::Bold)
  ///   .with_children(vec![Node::Text(CueStr::borrowed("text"))]);
  /// assert_eq!(node.children().len(), 1);
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn with_children<D>(self, children: D) -> TagNode<'a, D> {
    TagNode {
      tag: self.tag,
      classes: self.classes,
      annotation: self.annotation,
      children,
    }
  }

  /// Sets the child nodes (same container type).
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Node, CueStr, Tag};
  ///
  /// let mut node = TagNode::new(Tag::Italic);
  /// node.set_children(vec![Node::Text(CueStr::borrowed("text"))]);
  /// assert_eq!(node.children().len(), 1);
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn set_children(&mut self, children: C) -> &mut Self {
    self.children = children;
    self
  }

  /// Consumes the node and returns its children container.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Node, CueStr, Tag};
  ///
  /// let node = TagNode::new(Tag::Bold)
  ///   .with_children(vec![Node::Text(CueStr::borrowed("text"))]);
  /// let children = node.into_children();
  /// assert_eq!(children.len(), 1);
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn into_children(self) -> C {
    self.children
  }
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "alloc", feature = "std"))))]
impl TagNode<'_> {
  /// Create a new `TagNode` with the given tag and no classes, annotation,
  /// or children.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let node = TagNode::new(Tag::Bold);
  /// assert_eq!(node.tag(), Tag::Bold);
  /// assert_eq!(node.classes_raw(), "");
  /// assert_eq!(node.annotation(), None);
  /// assert!(node.children().is_empty());
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(tag: Tag) -> Self {
    Self {
      tag,
      classes: "",
      annotation: None,
      children: Vec::new(),
    }
  }

  /// Create a new `TagNode` with the given tag and no classes, annotation,
  /// or children.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let node = TagNode::with_vec_capacity(Tag::Bold, 10);
  /// assert_eq!(node.tag(), Tag::Bold);
  /// assert_eq!(node.classes_raw(), "");
  /// assert_eq!(node.annotation(), None);
  /// assert!(node.children().is_empty());
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]

  pub fn with_vec_capacity(tag: Tag, cap: usize) -> Self {
    Self {
      tag,
      classes: "",
      annotation: None,
      children: Vec::with_capacity(cap),
    }
  }
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "alloc", feature = "std"))))]
impl<'a, C: AsRef<[Node<'a>]>> fmt::Display for TagNode<'a, C> {
  /// Serializes the tag node to WebVTT cue text markup.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{Annotation, TagNode, Node, CueStr, Tag};
  ///
  /// let node = TagNode::new(Tag::Bold)
  ///   .with_children(vec![Node::Text(CueStr::borrowed("hello"))]);
  /// assert_eq!(node.to_string(), "<b>hello</b>");
  ///
  /// let node = TagNode::new(Tag::Lang)
  ///   .with_annotation(Some(Annotation::new("en")))
  ///   .with_children(vec![Node::Text(CueStr::borrowed("world"))]);
  /// assert_eq!(node.to_string(), "<lang en>world</lang>");
  ///
  /// let node = TagNode::new(Tag::Class)
  ///   .with_classes("loud.important");
  /// assert_eq!(node.to_string(), "<c.loud.important></c>");
  /// # }
  /// ```
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // Opening tag: <tag.classes annotation>
    write!(f, "<{}", self.tag)?;
    if !self.classes.is_empty() {
      write!(f, ".{}", self.classes)?;
    }
    // The annotation goes back as the cue spelled it. Its normalized form may
    // hold a `>` that a `&gt;` stood for, which would end the tag here.
    if let Some(annotation) = &self.annotation {
      write!(f, " {}", annotation.as_raw())?;
    }
    f.write_str(">")?;

    // Children
    for child in self.children.as_ref() {
      write!(f, "{}", child)?;
    }

    // Closing tag: </tag>
    write!(f, "</{}>", self.tag)
  }
}

/// The maximum cue text nesting depth [`CueText::parse`] accepts by default.
///
/// WebVTT places no bound on how deeply a cue payload may nest, and the depth
/// is chosen by the file rather than by the caller. A tree is walked
/// recursively by [`Clone`], [`PartialEq`], [`fmt::Debug`], [`fmt::Display`]
/// and by the compiler's own drop glue, so an unbounded tree lets a hostile
/// cue overflow the stack — an abort, which no caller can catch. Bounding the
/// tree bounds every one of those walks at once.
///
/// The value is chosen from both ends. It is far above what the format uses:
/// WebVTT's whole tag vocabulary (`<v>`, `<lang>`, `<c>`, `<b>`, `<i>`, `<u>`,
/// `<ruby>`, `<rt>`) is eight tags, and the deepest cue in this crate's entire
/// fixture corpus — 107 264 cue bodies, WPT and real-world — nests three deep.
/// And it is far below what a small thread can pay: at this depth the most
/// expensive walk needs roughly 50 KiB of stack in an *unoptimized* build
/// (measured on `aarch64-apple-darwin`; optimized builds cost far less), which
/// the regression suite holds to a 128 KiB thread — a sixteenth of the 2 MiB a
/// Rust thread is given by default.
#[cfg(any(feature = "alloc", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "alloc", feature = "std"))))]
pub const DEFAULT_MAX_DEPTH: usize = 16;

/// Options that control how raw cue text is turned into a [`CueText`] tree.
///
/// ```rust
/// # #[cfg(any(feature = "alloc", feature = "std"))]
/// # {
/// use fasrt::vtt::cue::{DEFAULT_MAX_DEPTH, Options};
///
/// assert_eq!(Options::new().max_depth(), DEFAULT_MAX_DEPTH);
/// # }
/// ```
///
/// With the `serde` feature, this type implements [`serde::Serialize`] and
/// [`serde::Deserialize`] as `{"max_depth": <usize>}`; a missing field takes
/// its value from [`Options::default`] ([`DEFAULT_MAX_DEPTH`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case", default))]
#[cfg(any(feature = "alloc", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "alloc", feature = "std"))))]
pub struct Options {
  /// The deepest the parsed tree may nest.
  max_depth: usize,
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "alloc", feature = "std"))))]
impl Options {
  /// The default options, bounding nesting at [`DEFAULT_MAX_DEPTH`].
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{DEFAULT_MAX_DEPTH, Options};
  ///
  /// assert_eq!(Options::new().max_depth(), DEFAULT_MAX_DEPTH);
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new() -> Self {
    Self {
      max_depth: DEFAULT_MAX_DEPTH,
    }
  }

  /// Returns the deepest the parsed tree may nest.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::Options;
  ///
  /// assert_eq!(Options::new().with_max_depth(8).max_depth(), 8);
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn max_depth(&self) -> usize {
    self.max_depth
  }

  /// Sets the deepest the parsed tree may nest (builder pattern).
  ///
  /// `0` keeps the text and drops every tag.
  ///
  /// Raising the limit raises the stack every recursive walk of the tree
  /// costs, and a walk that overflows the stack is an abort no caller can
  /// catch — so the limit is a stack budget, not a taste. Per level of depth,
  /// in an unoptimized build on `aarch64-apple-darwin`: [`fmt::Display`] about
  /// 3.7 KiB, [`fmt::Debug`], [`Clone`] and [`PartialEq`] about 1 KiB each,
  /// and construction and drop under a tenth of that. Optimized builds cost
  /// far less. Multiply by the stack the thinnest thread that will touch the
  /// tree is given, and leave room for the caller's own frames.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::Options;
  ///
  /// let opts = Options::new().with_max_depth(4);
  /// assert_eq!(opts.max_depth(), 4);
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with_max_depth(mut self, max_depth: usize) -> Self {
    self.max_depth = max_depth;
    self
  }

  /// Sets the deepest the parsed tree may nest.
  ///
  /// See [`with_max_depth`](Self::with_max_depth) for the cost of raising it.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::Options;
  ///
  /// let mut opts = Options::new();
  /// opts.set_max_depth(4);
  /// assert_eq!(opts.max_depth(), 4);
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn set_max_depth(&mut self, max_depth: usize) -> &mut Self {
    self.max_depth = max_depth;
    self
  }
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl Default for Options {
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn default() -> Self {
    Self::new()
  }
}

/// A WebVTT cue text DOM tree, generic over its children container.
///
/// The default container is `Vec<Node<'a>>`, returned by [`parse`].
/// For allocation-free writing you can use `[Node; N]` or `&[Node]`
/// instead.
///
/// A tree returned by [`parse`] nests no deeper than
/// [`Options::max_depth`], so walking it recursively — as [`Clone`],
/// [`PartialEq`], [`fmt::Debug`], [`fmt::Display`] and the drop glue all do —
/// costs bounded stack. A tree assembled by hand carries no such bound.
///
/// [`parse`]: CueText::parse
///
/// # Example
///
/// ```rust
/// # #[cfg(any(feature = "alloc", feature = "std"))]
/// # {
/// use fasrt::vtt::cue::{CueText, Tag, Node, CueStr};
///
/// let tree = CueText::parse("<b>hello</b> world");
/// assert_eq!(tree.children().len(), 2);
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(any(feature = "alloc", feature = "std"))]
pub struct CueText<'a, C = Vec<Node<'a>>> {
  children: C,
  _marker: core::marker::PhantomData<&'a ()>,
}

/// A WebVTT cue text DOM tree, generic over its children container.
///
/// The default container is `Vec<Node<'a>>`, returned by [`parse`].
/// For allocation-free writing you can use `[Node; N]` or `&[Node]`
/// instead.
///
/// [`parse`]: CueText::parse
///
/// # Example
///
/// ```rust
/// # #[cfg(any(feature = "alloc", feature = "std"))]
/// # {
/// use fasrt::vtt::cue::{CueText, Tag, Node, CueStr};
///
/// let tree = CueText::parse("<b>hello</b> world");
/// assert_eq!(tree.children().len(), 2);
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(not(any(feature = "alloc", feature = "std")))]
pub struct CueText<'a, C> {
  children: C,
  _marker: core::marker::PhantomData<&'a ()>,
}

impl<C> CueText<'_, C> {
  /// Create a new `CueText` with the given children container.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{CueText, Node, CueStr, TagNode, Tag};
  ///
  /// // Vec (heap-allocated)
  /// let tree = CueText::new(vec![
  ///   Node::Tag(TagNode::new(Tag::Bold)
  ///     .with_children(vec![Node::Text(CueStr::borrowed("hello"))])),
  ///   Node::Text(CueStr::borrowed(" world")),
  /// ]);
  /// assert_eq!(tree.to_string(), "<b>hello</b> world");
  ///
  /// // Fixed-size array (no allocation)
  /// let tree = CueText::new([
  ///   Node::Text(CueStr::borrowed("hello world")),
  /// ]);
  /// assert_eq!(tree.to_string(), "hello world");
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new(children: C) -> Self {
    Self {
      children,
      _marker: core::marker::PhantomData,
    }
  }

  /// Returns the root children of the DOM tree.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::CueText;
  ///
  /// let tree = CueText::parse("hello");
  /// assert_eq!(tree.children().len(), 1);
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn children(&self) -> &C {
    &self.children
  }

  /// Returns a mutable reference to the root children.
  ///
  /// Only available on `Vec`-based `CueText` (the default).
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{CueText, Node, CueStr};
  ///
  /// let mut tree = CueText::new(vec![]);
  /// tree.children_mut().push(Node::Text(CueStr::borrowed("hello")));
  /// assert_eq!(tree.children().len(), 1);
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn children_mut(&mut self) -> &mut C {
    &mut self.children
  }

  /// Consumes the `CueText` and returns the children container.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn into_children(self) -> C {
    self.children
  }
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "alloc", feature = "std"))))]
impl<'a, C> CueText<'a, C> {
  /// Returns the root children of the DOM tree.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::CueText;
  ///
  /// let tree = CueText::parse("hello");
  /// assert_eq!(tree.children_slice().len(), 1);
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn children_slice(&self) -> &[Node<'a>]
  where
    C: AsRef<[Node<'a>]>,
  {
    self.children.as_ref()
  }

  /// Returns a mutable reference to the root children.
  ///
  /// Only available on `Vec`-based `CueText` (the default).
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{CueText, Node, CueStr};
  ///
  /// let mut tree = CueText::new(vec![]);
  /// tree.children_mut().push(Node::Text(CueStr::borrowed("hello")));
  /// tree.children_slice_mut()[0] = Node::Text(CueStr::borrowed("hi"));
  /// assert_eq!(tree.children_slice().len(), 1);
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn children_slice_mut(&mut self) -> &mut [Node<'a>]
  where
    C: AsMut<[Node<'a>]>,
  {
    self.children.as_mut()
  }

  /// Walks the whole tree in document order, pairing every node with the
  /// language W3C WebVTT §6.4 makes applicable to it.
  ///
  /// §6.4 keeps a language stack and stamps each node it attaches with the
  /// stack's top entry. That stack is exactly the chain of enclosing `<lang>`
  /// nodes — `<lang>` is the only tag that pushes, and the only end tag that
  /// pops is the one that closes a `<lang>` — so the applicable language is
  /// derived here rather than stored on the node: a derived answer cannot go
  /// stale when a tree is edited through [`children_mut`](Self::children_mut).
  ///
  /// The language is the annotation of the nearest enclosing `<lang>`, and
  /// `None` where none encloses the node — §6.4's empty stack. A `<lang>` node
  /// carries the language it declares, since §6.4 pushes before it attaches.
  /// The value is §6.4's annotation, normalized as
  /// [`TagNode::declared_language`] documents.
  ///
  /// `None` and `Some("")` are different answers and a caller must not merge
  /// them. `None` is an empty language stack: nothing in the cue says anything
  /// about this node's language, so a fallback from outside the cue — the
  /// track's language, say — applies to it. `Some("")` is an annotation-less
  /// `<lang>`, which pushed the empty string: the cue has said, explicitly,
  /// that this subtree is in no known language, and that clears the fallback
  /// rather than deferring to it.
  ///
  /// The walk is iterative, so it costs no stack in the depth of the tree.
  ///
  /// # It answers for the tree it is given
  ///
  /// A `<lang>` that [`Options::max_depth`] dropped is not in the tree, so its
  /// scope is not either: `parse_with("<b><lang fr>x</lang>y</b>", max_depth =
  /// 1)` builds `<b>xy</b>`, and both text nodes read as `""`. This is the
  /// depth bound's documented lossiness — the over-deep tag is discarded
  /// exactly as an unrecognized tag is, and it takes its language with it,
  /// just as it takes its classes and its markup. A caller who needs the
  /// applicable language to mean §6.4's unbounded answer should parse with
  /// [`try_parse`] or [`try_parse_with`], which refuse input that deep rather
  /// than returning a tree that dropped part of it.
  ///
  /// [`try_parse`]: CueText::try_parse
  /// [`try_parse_with`]: CueText::try_parse_with
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{CueText, Node};
  ///
  /// let tree = CueText::parse("<lang ja>ハロー<b>ワールド</b></lang>!");
  /// let text: Vec<_> = tree
  ///   .nodes_with_language()
  ///   .filter_map(|(node, language)| match node {
  ///     Node::Text(text) => Some((text.normalize(), language)),
  ///     _ => None,
  ///   })
  ///   .collect();
  /// assert_eq!(
  ///   text,
  ///   [("ハロー", Some("ja")), ("ワールド", Some("ja")), ("!", None)],
  /// );
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn nodes_with_language(&self) -> NodesWithLanguage<'_, 'a>
  where
    C: AsRef<[Node<'a>]>,
  {
    NodesWithLanguage::new(self.children.as_ref())
  }
}

/// A depth-first walk over a cue text tree, pairing each node with its
/// applicable language.
///
/// Created by [`CueText::nodes_with_language`], which documents how the
/// language is derived.
#[derive(Debug, Clone)]
#[cfg(any(feature = "alloc", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "alloc", feature = "std"))))]
pub struct NodesWithLanguage<'t, 'a> {
  /// One frame per open ancestor, each holding the siblings still to visit
  /// and the language applicable inside that ancestor.
  stack: Vec<(core::slice::Iter<'t, Node<'a>>, Option<&'t str>)>,
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl<'t, 'a> NodesWithLanguage<'t, 'a> {
  fn new(nodes: &'t [Node<'a>]) -> Self {
    // §6.4 starts with an empty language stack, which has no top entry —
    // distinct from a top entry that is the empty string.
    Self {
      stack: std::vec![(nodes.iter(), None)],
    }
  }
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "alloc", feature = "std"))))]
impl<'t, 'a> Iterator for NodesWithLanguage<'t, 'a> {
  /// A node and the language applicable to it: the top of §6.4's language
  /// stack, or `None` while that stack is empty.
  type Item = (&'t Node<'a>, Option<&'t str>);

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      let (siblings, language) = self.stack.last_mut()?;
      let language = *language;
      match siblings.next() {
        // A tag node may declare a language for its own subtree; §6.4 pushes
        // before attaching, so the node itself already carries it. `or` keeps
        // an annotation-less `<lang>`'s `Some("")` — it pushed, and an empty
        // push is not the same as no push.
        Some(node @ Node::Tag(tag)) => {
          let inner = tag.declared_language().or(language);
          self.stack.push((tag.children().iter(), inner));
          return Some((node, inner));
        }
        Some(node) => return Some((node, language)),
        None => {
          self.stack.pop();
        }
      }
    }
  }
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl core::iter::FusedIterator for NodesWithLanguage<'_, '_> {}

#[cfg(any(feature = "alloc", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "alloc", feature = "std"))))]
impl<'a> CueText<'a> {
  /// Parse raw cue text into a DOM tree, bounding nesting at
  /// [`DEFAULT_MAX_DEPTH`].
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{CueText, Node, Tag};
  ///
  /// let tree = CueText::parse("<b>hello</b> world");
  /// assert_eq!(tree.children().len(), 2);
  /// assert!(matches!(&tree.children()[0], Node::Tag(t) if t.tag() == Tag::Bold));
  /// assert!(matches!(&tree.children()[1], Node::Text(t) if t.normalize() == " world"));
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn parse(input: &'a str) -> Self {
    Self::parse_with(input, Options::new())
  }

  /// Parse raw cue text into a DOM tree with the given options.
  ///
  /// Markup nested past [`Options::max_depth`] is dropped: the tag itself is
  /// discarded, exactly as an unrecognized tag already is, while its text is
  /// kept and its end tag is still consumed — so the cue's text is complete
  /// and the structure that follows the over-deep run is the structure an
  /// unbounded parse would have produced. Use [`try_parse_with`] to be told
  /// that the input was that deep instead.
  ///
  /// [`try_parse_with`]: CueText::try_parse_with
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{CueText, Options};
  ///
  /// let tree = CueText::parse_with("<b><i>deep</i></b>", Options::new().with_max_depth(1));
  /// // `<b>` fits; `<i>` does not, so only its text survives.
  /// assert_eq!(tree.to_string(), "<b>deep</b>");
  /// # }
  /// ```
  pub fn parse_with(input: &'a str, options: Options) -> Self {
    Self::build(input, options.max_depth(), false).0
  }

  /// Parse raw cue text into a DOM tree, refusing input that nests deeper than
  /// [`DEFAULT_MAX_DEPTH`].
  ///
  /// # Errors
  ///
  /// Returns [`MaxDepthExceededError`] when the input nests deeper than the
  /// limit.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::CueText;
  ///
  /// assert!(CueText::try_parse("<b>hello</b>").is_ok());
  /// assert!(CueText::try_parse(&"<i>".repeat(20_000)).is_err());
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn try_parse(input: &'a str) -> Result<Self, MaxDepthExceededError> {
    Self::try_parse_with(input, Options::new())
  }

  /// Parse raw cue text into a DOM tree with the given options, refusing input
  /// that nests deeper than [`Options::max_depth`].
  ///
  /// # Errors
  ///
  /// Returns [`MaxDepthExceededError`] when the input nests deeper than the
  /// limit.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{CueText, Options};
  ///
  /// let opts = Options::new().with_max_depth(1);
  /// assert!(CueText::try_parse_with("<b>hello</b>", opts).is_ok());
  ///
  /// let err = CueText::try_parse_with("<b><i>deep</i></b>", opts).unwrap_err();
  /// assert_eq!(err.max_depth(), 1);
  /// # }
  /// ```
  pub fn try_parse_with(input: &'a str, options: Options) -> Result<Self, MaxDepthExceededError> {
    match Self::build(input, options.max_depth(), true) {
      (tree, false) => Ok(tree),
      (_, true) => Err(MaxDepthExceededError::new(options.max_depth())),
    }
  }

  /// Builds the tree, reporting alongside it whether `max_depth` was exceeded.
  ///
  /// With `stop_when_exceeded` the walk returns at the first tag past the
  /// limit, so a caller that is about to refuse the input does no work beyond
  /// it; the partial tree it returns is meant to be discarded.
  fn build(input: &'a str, max_depth: usize, stop_when_exceeded: bool) -> (Self, bool) {
    let mut builder = Builder::new(max_depth);
    let mut exceeded = false;

    for token in CueParser::new(input) {
      match token {
        CueToken::Text(text) => builder.push_leaf(Node::Text(text)),
        CueToken::Timestamp(ts) => builder.push_leaf(Node::Timestamp(ts)),
        CueToken::StartTag {
          tag,
          classes,
          annotation,
        } => {
          // §6.4, start tag, tag name "rt": "If current is a WebVTT Ruby
          // Object, then attach a WebVTT Ruby Text Object." The test is on
          // `current`, not on having a `<ruby>` ancestor, so `<ruby><b><rt>`
          // attaches nothing — the token is ignored and `current` stays
          // `<b>`. Every other recognized tag attaches unconditionally.
          if tag == Tag::RubyText && builder.current_tag() != Some(Tag::Ruby) {
            continue;
          }
          if !builder.open(tag, classes, annotation) {
            exceeded = true;
            if stop_when_exceeded {
              break;
            }
          }
        }
        CueToken::EndTag(tag) => {
          // §6.4, end tag. The seven (tag name, current-node class) pairs that
          // mean "let current be the parent node of current" — and the "lang"
          // clause that follows them, which differs only in also popping the
          // language stack this parser does not model — all say one thing
          // about the tree: the end tag names the current node's own class.
          match builder.current_tag() {
            Some(current) if current == tag => builder.close_current(),
            // "Otherwise, if the tag name of the end tag token is "ruby" and
            // current is a WebVTT Ruby Text Object, then let current be the
            // parent node of the parent node of current." A Ruby Text Object
            // is attached only while current is a Ruby Object, so its parent
            // is always that `<ruby>`: one `</ruby>` closes both. This is the
            // only end tag that closes a node it does not name.
            Some(Tag::RubyText) if tag == Tag::Ruby => {
              builder.close_current();
              builder.close_current();
            }
            // "Otherwise, ignore the token." An unmatched end tag closes
            // nothing — in particular it does not close an open `<rt>`.
            _ => {}
          }
        }
      }
    }

    (Self::new(builder.finish()), exceeded)
  }
}

/// The working state of [`CueText::build`].
///
/// `stack` holds the ancestors that are materialized as [`TagNode`]s, and
/// never grows past `max_depth`. `over` holds the ancestors past that limit
/// as names only: their markup is dropped, but end-tag matching still sees the
/// nesting the input declared, so the algorithm below stays the W3C one and
/// only the deepest markup goes missing.
///
/// While `over` is non-empty the innermost *materialized* node is
/// `stack.last_mut()`, which is where text keeps landing — so an over-deep run
/// costs O(1) per token rather than splicing children up the tree.
#[cfg(any(feature = "alloc", feature = "std"))]
struct Builder<'a> {
  root: Vec<Node<'a>>,
  stack: Vec<TagNode<'a>>,
  over: Vec<Tag>,
  max_depth: usize,
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl<'a> Builder<'a> {
  fn new(max_depth: usize) -> Self {
    Self {
      root: Vec::new(),
      stack: Vec::new(),
      over: Vec::new(),
      max_depth,
    }
  }

  /// The tag of §6.4's `current` node, or `None` when `current` is the root.
  ///
  /// An over-deep tag materializes no node, but it is still the node the
  /// algorithm calls `current`, so `over` is consulted before `stack`.
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn current_tag(&self) -> Option<Tag> {
    match self.over.last() {
      Some(tag) => Some(*tag),
      None => self.stack.last().map(|node| node.tag()),
    }
  }

  /// Appends a leaf to the innermost materialized node.
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn push_leaf(&mut self, node: Node<'a>) {
    match self.stack.last_mut() {
      Some(parent) => parent.children_mut().push(node),
      None => self.root.push(node),
    }
  }

  /// Opens a tag. Returns `false` when it lands past `max_depth`, in which
  /// case only its name is kept and no node is materialized for it.
  fn open(&mut self, tag: Tag, classes: &'a str, annotation: Option<Annotation<'a>>) -> bool {
    if self.over.is_empty() && self.stack.len() < self.max_depth {
      self.stack.push(
        TagNode::new(tag)
          .with_classes(classes)
          .with_annotation(annotation),
      );
      true
    } else {
      self.over.push(tag);
      false
    }
  }

  /// Closes the innermost open tag, attaching its node to its parent. An
  /// over-deep tag has no node to attach: its children were appended to the
  /// innermost materialized node as they arrived, and stay there.
  fn close_current(&mut self) {
    if self.over.pop().is_some() {
      return;
    }
    if let Some(node) = self.stack.pop() {
      self.push_leaf(Node::Tag(node));
    }
  }

  /// Folds every still-open tag into its parent and returns the root children.
  fn finish(mut self) -> Vec<Node<'a>> {
    while let Some(node) = self.stack.pop() {
      let completed = Node::Tag(node);
      match self.stack.last_mut() {
        Some(parent) => parent.children_mut().push(completed),
        None => self.root.push(completed),
      }
    }
    self.root
  }
}

#[cfg(any(feature = "alloc", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "alloc", feature = "std"))))]
impl<'a, C: Nodes<'a>> fmt::Display for CueText<'a, C> {
  /// Serializes the cue text DOM tree to WebVTT cue text markup.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{CueText, Node, CueStr, TagNode, Tag};
  ///
  /// let tree = CueText::new(vec![
  ///   Node::Tag(TagNode::new(Tag::Bold)
  ///     .with_children(vec![Node::Text(CueStr::borrowed("hello"))])),
  ///   Node::Text(CueStr::borrowed(" world")),
  /// ]);
  /// assert_eq!(tree.to_string(), "<b>hello</b> world");
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for child in self.children.as_nodes() {
      write!(f, "{}", child)?;
    }
    Ok(())
  }
}

#[cfg(test)]
#[cfg(any(feature = "alloc", feature = "std"))]
mod tests {
  use super::*;

  #[test]
  fn tag_node_as_slice() {
    let node = TagNode::new(Tag::Bold);
    let slice: &[TagNode<'_>] = node.as_ref();
    assert_eq!(slice.len(), 1);
    assert_eq!(slice[0].tag(), Tag::Bold);
  }

  #[test]
  fn tag_node_as_mut_slice() {
    let mut node = TagNode::new(Tag::Italic);
    let slice: &mut [TagNode<'_>] = node.as_mut();
    assert_eq!(slice.len(), 1);
    assert_eq!(slice[0].tag(), Tag::Italic);
    slice[0].set_tag(Tag::Underline);
    assert_eq!(node.tag(), Tag::Underline);
  }

  #[test]
  fn tag_node_as_ref() {
    let node = TagNode::new(Tag::Class);
    let r: &TagNode<'_> = node.as_ref();
    assert_eq!(r.tag(), Tag::Class);
  }

  #[test]
  fn tag_node_as_mut() {
    let mut node = TagNode::new(Tag::Lang);
    let r: &mut TagNode<'_> = node.as_mut();
    assert_eq!(r.tag(), Tag::Lang);
    r.set_tag(Tag::Voice);
    assert_eq!(node.tag(), Tag::Voice);
  }
}
