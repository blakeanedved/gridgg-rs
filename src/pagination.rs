//! Relay-style cursor pagination.
//!
//! GRID connections are paged with `first`/`after` (forward) or
//! `last`/`before` (backward); the two directions must never be mixed, and a
//! page holds at most [`PageRequest::MAX_SIZE`] items. [`PageRequest`] makes
//! mixed requests unrepresentable, and [`paginate`] walks every page of a
//! connection as a [`Stream`].

use std::future::Future;

use futures_util::Stream;

use crate::error::{Error, Result};

/// Position information of a page (the `PageInfo` type of the Central Data
/// schema).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
pub struct PageInfo {
    /// Whether a page exists after this one.
    pub has_next_page: bool,
    /// Whether a page exists before this one.
    pub has_previous_page: bool,
    /// Cursor of the first edge of the page.
    pub start_cursor: Option<String>,
    /// Cursor of the last edge of the page.
    pub end_cursor: Option<String>,
}

/// Which page of a connection to fetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageRequest {
    /// Up to `first` items after the `after` cursor (or from the start).
    Forward {
        /// Page size, `1..=50`.
        first: u32,
        /// Cursor to start after; `None` for the first page.
        after: Option<String>,
    },
    /// Up to `last` items before the `before` cursor (or from the end).
    Backward {
        /// Page size, `1..=50`.
        last: u32,
        /// Cursor to end before; `None` for the last page.
        before: Option<String>,
    },
}

impl Default for PageRequest {
    /// The first page with the maximum size.
    fn default() -> Self {
        Self::Forward {
            first: Self::MAX_SIZE,
            after: None,
        }
    }
}

impl PageRequest {
    /// The largest page size GRID accepts.
    pub const MAX_SIZE: u32 = 50;

    /// Up to `first` items after `after` (or from the start).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`] if `first` is not in `1..=50`.
    pub fn forward(first: u32, after: Option<String>) -> Result<Self> {
        validate_size(first)?;
        Ok(Self::Forward { first, after })
    }

    /// Up to `last` items before `before` (or from the end).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`] if `last` is not in `1..=50`.
    pub fn backward(last: u32, before: Option<String>) -> Result<Self> {
        validate_size(last)?;
        Ok(Self::Backward { last, before })
    }

    /// Checks that the page size is in `1..=50`. Useful when the variants
    /// were constructed directly.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`] if the page size is out of range.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Forward { first: size, .. } | Self::Backward { last: size, .. } => {
                validate_size(*size)
            }
        }
    }

    /// The `first` argument, if paging forward.
    #[must_use]
    pub fn first(&self) -> Option<u32> {
        match self {
            Self::Forward { first, .. } => Some(*first),
            Self::Backward { .. } => None,
        }
    }

    /// The `after` argument, if paging forward from a cursor.
    #[must_use]
    pub fn after(&self) -> Option<&str> {
        match self {
            Self::Forward { after, .. } => after.as_deref(),
            Self::Backward { .. } => None,
        }
    }

    /// The `last` argument, if paging backward.
    #[must_use]
    pub fn last(&self) -> Option<u32> {
        match self {
            Self::Backward { last, .. } => Some(*last),
            Self::Forward { .. } => None,
        }
    }

    /// The `before` argument, if paging backward from a cursor.
    #[must_use]
    pub fn before(&self) -> Option<&str> {
        match self {
            Self::Backward { before, .. } => before.as_deref(),
            Self::Forward { .. } => None,
        }
    }

    /// The request for the page after `info` in the same direction, or
    /// `None` when there is no such page, the server sent no cursor, or the
    /// cursor equals this request's (which would fetch the same page again).
    #[must_use]
    pub fn next(&self, info: &PageInfo) -> Option<Self> {
        match self {
            Self::Forward { first, after } if info.has_next_page => {
                let cursor = info.end_cursor.clone()?;
                (after.as_ref() != Some(&cursor)).then_some(Self::Forward {
                    first: *first,
                    after: Some(cursor),
                })
            }
            Self::Backward { last, before } if info.has_previous_page => {
                let cursor = info.start_cursor.clone()?;
                (before.as_ref() != Some(&cursor)).then_some(Self::Backward {
                    last: *last,
                    before: Some(cursor),
                })
            }
            _ => None,
        }
    }
}

fn validate_size(size: u32) -> Result<()> {
    if (1..=PageRequest::MAX_SIZE).contains(&size) {
        Ok(())
    } else {
        Err(Error::Config(format!(
            "page size must be between 1 and {}, got {size}",
            PageRequest::MAX_SIZE
        )))
    }
}

/// One fetched page: its items in server order, its [`PageInfo`] and, when
/// the connection reports it, the total number of items across all pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    /// The page's nodes, in the order the server returned them.
    pub items: Vec<T>,
    /// Position information used to request the following page.
    pub page_info: PageInfo,
    /// The connection's `totalCount` (all items matching the query, not just
    /// this page), or `None` if the connection does not report one.
    pub total_count: Option<u64>,
}

/// Walks a connection page by page, starting at `start`, and yields every
/// item.
///
/// `fetch` is called with each [`PageRequest`] in turn. Paging continues in
/// `start`'s direction while the server reports another page and supplies
/// a cursor different from the previous one (so a server that repeats its
/// cursor cannot make the stream loop forever). Items are yielded in server
/// order within each page. The
/// stream ends after the first error, which it yields.
pub fn paginate<T, F, Fut>(start: PageRequest, mut fetch: F) -> impl Stream<Item = Result<T>>
where
    F: FnMut(PageRequest) -> Fut,
    Fut: Future<Output = Result<Page<T>>>,
{
    async_stream::stream! {
        if let Err(err) = start.validate() {
            yield Err(err);
            return;
        }
        let mut request = Some(start);
        while let Some(current) = request.take() {
            match fetch(current.clone()).await {
                Ok(page) => {
                    request = current.next(&page.page_info);
                    for item in page.items {
                        yield Ok(item);
                    }
                }
                Err(err) => {
                    yield Err(err);
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures_util::{StreamExt, TryStreamExt};

    use super::*;

    fn info(has_next: bool, end: Option<&str>) -> PageInfo {
        PageInfo {
            has_next_page: has_next,
            has_previous_page: false,
            start_cursor: None,
            end_cursor: end.map(str::to_owned),
        }
    }

    #[test]
    fn validates_page_size() {
        assert!(PageRequest::forward(0, None).is_err());
        assert!(PageRequest::forward(51, None).is_err());
        assert!(PageRequest::backward(50, Some("c".into())).is_ok());
        let req = PageRequest::forward(10, Some("c".into())).unwrap();
        assert_eq!((req.first(), req.after()), (Some(10), Some("c")));
        assert_eq!((req.last(), req.before()), (None, None));
    }

    #[test]
    fn next_stays_in_direction() {
        let backward = PageRequest::backward(5, None).unwrap();
        let page = PageInfo {
            has_next_page: true,
            has_previous_page: true,
            start_cursor: Some("s".into()),
            end_cursor: Some("e".into()),
        };
        assert_eq!(
            backward.next(&page),
            Some(PageRequest::Backward {
                last: 5,
                before: Some("s".into())
            })
        );
        assert_eq!(PageRequest::default().next(&info(true, None)), None);
    }

    #[tokio::test]
    async fn walks_all_pages() {
        let mut seen = Vec::new();
        let items: Vec<u32> = paginate(PageRequest::forward(2, None).unwrap(), |req| {
            seen.push(req.after().map(str::to_owned));
            let page = match req.after() {
                None => Page {
                    items: vec![1, 2],
                    page_info: info(true, Some("a")),
                    total_count: None,
                },
                Some("a") => Page {
                    items: vec![3, 4],
                    page_info: info(true, Some("b")),
                    total_count: None,
                },
                _ => Page {
                    items: vec![5],
                    page_info: info(false, Some("c")),
                    total_count: None,
                },
            };
            async move { Ok(page) }
        })
        .try_collect()
        .await
        .unwrap();
        assert_eq!(items, [1, 2, 3, 4, 5]);
        assert_eq!(seen, [None, Some("a".into()), Some("b".into())]);
    }

    #[test]
    fn next_rejects_a_repeated_cursor() {
        let forward = PageRequest::forward(5, Some("a".into())).unwrap();
        assert_eq!(forward.next(&info(true, Some("a"))), None);
        let backward = PageRequest::backward(5, Some("s".into())).unwrap();
        let page = PageInfo {
            has_next_page: false,
            has_previous_page: true,
            start_cursor: Some("s".into()),
            end_cursor: None,
        };
        assert_eq!(backward.next(&page), None);
        let page = PageInfo {
            start_cursor: None,
            ..page
        };
        assert_eq!(backward.next(&page), None);
    }

    #[tokio::test]
    async fn ends_when_the_server_repeats_its_cursor() {
        let mut calls = 0;
        let items: Vec<u32> = paginate(PageRequest::forward(2, None).unwrap(), |_| {
            calls += 1;
            assert!(calls <= 2, "fetched the same page again");
            let page = Page {
                items: vec![calls],
                page_info: info(true, Some("same")),
                total_count: None,
            };
            async move { Ok(page) }
        })
        .try_collect()
        .await
        .unwrap();
        assert_eq!(items, [1, 2]);
    }

    #[tokio::test]
    async fn stops_after_error() {
        let stream = paginate(PageRequest::default(), |_| async {
            Err::<Page<u32>, _>(Error::Config("boom".into()))
        });
        let results: Vec<_> = stream.collect().await;
        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
    }

    #[tokio::test]
    async fn invalid_start_is_an_error() {
        let start = PageRequest::Forward {
            first: 100,
            after: None,
        };
        let results: Vec<Result<u32>> = paginate(start, |_| async {
            unreachable!("fetch must not be called")
        })
        .collect()
        .await;
        assert!(matches!(results.as_slice(), [Err(Error::Config(_))]));
    }
}
