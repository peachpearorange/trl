/// Takes any tokens and expands to nothing — useful for commenting out
/// large blocks of code without prefixing every line with `//`.
#[allow(unused_macros)]
macro_rules! comment {
  ($($t:tt)*) => {};
}

pub fn map<I, F, B>(f: F, iter: I) -> impl Iterator<Item = B>
where
  I: IntoIterator,
  F: FnMut(I::Item) -> B
{
  iter.into_iter().map(f)
}

pub fn filter<I, F>(f: F, iter: I) -> impl Iterator<Item = I::Item>
where
  I: IntoIterator,
  F: FnMut(&I::Item) -> bool
{
  iter.into_iter().filter(f)
}

pub fn filter_map<I, F, B>(f: F, iter: I) -> impl Iterator<Item = B>
where
  I: IntoIterator,
  F: FnMut(I::Item) -> Option<B>
{
  iter.into_iter().filter_map(f)
}

pub fn find<I, F>(f: F, iter: I) -> Option<I::Item>
where
  I: IntoIterator,
  F: FnMut(&I::Item) -> bool
{
  iter.into_iter().find(f)
}

pub fn find_map<I, F, B>(f: F, iter: I) -> Option<B>
where
  I: IntoIterator,
  F: FnMut(I::Item) -> Option<B>
{
  iter.into_iter().find_map(f)
}

pub fn fold<I, B, F>(f: F, init: B, iter: I) -> B
where
  I: IntoIterator,
  F: FnMut(B, I::Item) -> B
{
  iter.into_iter().fold(init, f)
}

pub fn any<I, F>(f: F, iter: I) -> bool
where
  I: IntoIterator,
  F: FnMut(I::Item) -> bool
{
  iter.into_iter().any(f)
}

pub fn all<I, F>(f: F, iter: I) -> bool
where
  I: IntoIterator,
  F: FnMut(I::Item) -> bool
{
  iter.into_iter().all(f)
}

pub fn flat_map<I, F, U>(f: F, iter: I) -> impl Iterator<Item = U::Item>
where
  I: IntoIterator,
  F: FnMut(I::Item) -> U,
  U: IntoIterator
{
  iter.into_iter().flat_map(f)
}

pub fn collect_vec<I>(iter: I) -> Vec<I::Item>
where
  I: IntoIterator
{
  iter.into_iter().collect()
}

pub fn mapv<I, F, B>(f: F, iter: I) -> Vec<B>
where
  I: IntoIterator,
  F: FnMut(I::Item) -> B
{
  iter.into_iter().map(f).collect()
}
