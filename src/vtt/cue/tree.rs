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
  annotation: Option<&'a str>,
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
  annotation: Option<&'a str>,
  children: C,
}

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

  /// Returns the dot-separated class names (e.g., `"loud.important"`),
  /// empty if none.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let node = TagNode::new(Tag::Class).with_classes("loud.important");
  /// assert_eq!(node.classes(), "loud.important");
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn classes(&self) -> &'a str {
    self.classes
  }

  /// Sets the class names (builder pattern).
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let node = TagNode::new(Tag::Class).with_classes("highlight");
  /// assert_eq!(node.classes(), "highlight");
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn with_classes(mut self, classes: &'a str) -> Self {
    self.classes = classes;
    self
  }

  /// Sets the class names.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let mut node = TagNode::new(Tag::Class);
  /// node.set_classes("loud");
  /// assert_eq!(node.classes(), "loud");
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn set_classes(&mut self, classes: &'a str) -> &mut Self {
    self.classes = classes;
    self
  }

  /// Returns the annotation text (for `<v>` and `<lang>`), `None` if
  /// absent.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let node = TagNode::new(Tag::Voice).with_annotation(Some("Speaker"));
  /// assert_eq!(node.annotation(), Some("Speaker"));
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn annotation(&self) -> Option<&'a str> {
    self.annotation
  }

  /// Sets the annotation text (builder pattern).
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let node = TagNode::new(Tag::Lang).with_annotation(Some("en"));
  /// assert_eq!(node.annotation(), Some("en"));
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn with_annotation(mut self, annotation: Option<&'a str>) -> Self {
    self.annotation = annotation;
    self
  }

  /// Sets the annotation text.
  ///
  /// ```rust
  /// # #[cfg(any(feature = "alloc", feature = "std"))]
  /// # {
  /// use fasrt::vtt::cue::{TagNode, Tag};
  ///
  /// let mut node = TagNode::new(Tag::Voice);
  /// node.set_annotation(Some("Roger"));
  /// assert_eq!(node.annotation(), Some("Roger"));
  /// # }
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub fn set_annotation(&mut self, annotation: Option<&'a str>) -> &mut Self {
    self.annotation = annotation;
    self
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
  /// assert_eq!(node.classes(), "");
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
  /// assert_eq!(node.classes(), "");
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
  /// use fasrt::vtt::cue::{TagNode, Node, CueStr, Tag};
  ///
  /// let node = TagNode::new(Tag::Bold)
  ///   .with_children(vec![Node::Text(CueStr::borrowed("hello"))]);
  /// assert_eq!(node.to_string(), "<b>hello</b>");
  ///
  /// let node = TagNode::new(Tag::Lang)
  ///   .with_annotation(Some("en"))
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
    if let Some(ann) = self.annotation {
      write!(f, " {}", ann)?;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
}

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
          // Per spec: <rt> is only allowed inside <ruby>
          if tag == Tag::RubyText && !builder.in_ruby() {
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
          // W3C WebVTT spec §6.4 end tag processing:

          // 1. </rt> requires a <ruby> ancestor
          if tag == Tag::RubyText && !builder.in_ruby() {
            continue;
          }

          // 2. Generate implied end tags: while top of stack is <rt>, close it
          while builder.current_tag() == Some(Tag::RubyText) {
            builder.close_current();
          }

          // 3. If current node matches, pop it
          if builder.current_tag() == Some(tag) {
            builder.close_current();
          }
          // Otherwise: end tag is ignored (spec says jump to next token)
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
  /// The number of open `<ruby>` ancestors across both stacks — the O(1) form
  /// of the spec's "has a `<ruby>` ancestor" test.
  ///
  /// Every push through [`open`](Self::open) that increments this is matched
  /// by exactly one pop through [`close_current`](Self::close_current) that
  /// decrements it, so the count cannot go below zero.
  ruby_depth: usize,
  max_depth: usize,
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl<'a> Builder<'a> {
  fn new(max_depth: usize) -> Self {
    Self {
      root: Vec::new(),
      stack: Vec::new(),
      over: Vec::new(),
      ruby_depth: 0,
      max_depth,
    }
  }

  /// The innermost open tag, or `None` at the root.
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn current_tag(&self) -> Option<Tag> {
    match self.over.last() {
      Some(tag) => Some(*tag),
      None => self.stack.last().map(|node| node.tag()),
    }
  }

  #[cfg_attr(not(tarpaulin), inline(always))]
  fn in_ruby(&self) -> bool {
    self.ruby_depth > 0
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
  fn open(&mut self, tag: Tag, classes: &'a str, annotation: Option<&'a str>) -> bool {
    if tag == Tag::Ruby {
      self.ruby_depth += 1;
    }

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
    let tag = match self.over.pop() {
      Some(tag) => tag,
      None => {
        let Some(node) = self.stack.pop() else { return };
        let tag = node.tag();
        self.push_leaf(Node::Tag(node));
        tag
      }
    };

    if tag == Tag::Ruby {
      self.ruby_depth -= 1;
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
