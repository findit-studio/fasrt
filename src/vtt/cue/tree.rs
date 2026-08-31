#[cfg(any(feature = "alloc", feature = "std"))]
use derive_more::{TryUnwrap, Unwrap};

use super::*;

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
  /// Parse raw cue text into a DOM tree.
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
  pub fn parse(input: &'a str) -> Self {
    let mut root_children = Vec::new();
    let mut stack: Vec<TagNode<'a>> = Vec::new();

    for token in CueParser::new(input) {
      match token {
        CueToken::Text(text) => {
          let node = Node::Text(text);
          if let Some(parent) = stack.last_mut() {
            parent.children_mut().push(node);
          } else {
            root_children.push(node);
          }
        }
        CueToken::Timestamp(ts) => {
          let node = Node::Timestamp(ts);
          if let Some(parent) = stack.last_mut() {
            parent.children_mut().push(node);
          } else {
            root_children.push(node);
          }
        }
        CueToken::StartTag {
          tag,
          classes,
          annotation,
        } => {
          // Per spec: <rt> is only allowed inside <ruby>
          if tag == Tag::RubyText && !stack.iter().any(|n| n.tag() == Tag::Ruby) {
            continue;
          }
          stack.push(
            TagNode::new(tag)
              .with_classes(classes)
              .with_annotation(annotation),
          );
        }
        CueToken::EndTag(tag) => {
          // W3C WebVTT spec §6.4 end tag processing:

          // 1. </rt> requires a <ruby> ancestor
          if tag == Tag::RubyText && !stack.iter().any(|n| n.tag() == Tag::Ruby) {
            continue;
          }

          // 2. Generate implied end tags: while top of stack is <rt>, close it
          while stack.last().is_some_and(|n| n.tag() == Tag::RubyText) {
            let rt = stack.pop().unwrap();
            let target = stack
              .last_mut()
              .map_or(&mut root_children, |p| p.children_mut());
            target.push(Node::Tag(rt));
          }

          // 3. If current node matches, pop it
          if let Some(node) = stack.pop_if(|n| n.tag() == tag) {
            let target = stack
              .last_mut()
              .map_or(&mut root_children, |p| p.children_mut());
            target.push(Node::Tag(node));
          }
          // Otherwise: end tag is ignored (spec says jump to next token)
        }
      }
    }

    // Any unclosed tags become root children (fold into parents)
    while let Some(node) = stack.pop() {
      let completed = Node::Tag(node);
      if let Some(parent) = stack.last_mut() {
        parent.children_mut().push(completed);
      } else {
        root_children.push(completed);
      }
    }

    Self {
      children: root_children,
      _marker: core::marker::PhantomData,
    }
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
