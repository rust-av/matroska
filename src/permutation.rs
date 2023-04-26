use nom::error::{ErrorKind, ParseError};
use nom::{Err, IResult, Parser};

use crate::ebml::Error;

macro_rules! succ (
  (0, $submac:ident ! ($($rest:tt)*)) => ($submac!(1, $($rest)*));
  (1, $submac:ident ! ($($rest:tt)*)) => ($submac!(2, $($rest)*));
  (2, $submac:ident ! ($($rest:tt)*)) => ($submac!(3, $($rest)*));
  (3, $submac:ident ! ($($rest:tt)*)) => ($submac!(4, $($rest)*));
  (4, $submac:ident ! ($($rest:tt)*)) => ($submac!(5, $($rest)*));
  (5, $submac:ident ! ($($rest:tt)*)) => ($submac!(6, $($rest)*));
  (6, $submac:ident ! ($($rest:tt)*)) => ($submac!(7, $($rest)*));
  (7, $submac:ident ! ($($rest:tt)*)) => ($submac!(8, $($rest)*));
  (8, $submac:ident ! ($($rest:tt)*)) => ($submac!(9, $($rest)*));
  (9, $submac:ident ! ($($rest:tt)*)) => ($submac!(10, $($rest)*));
  (10, $submac:ident ! ($($rest:tt)*)) => ($submac!(11, $($rest)*));
  (11, $submac:ident ! ($($rest:tt)*)) => ($submac!(12, $($rest)*));
  (12, $submac:ident ! ($($rest:tt)*)) => ($submac!(13, $($rest)*));
  (13, $submac:ident ! ($($rest:tt)*)) => ($submac!(14, $($rest)*));
  (14, $submac:ident ! ($($rest:tt)*)) => ($submac!(15, $($rest)*));
  (15, $submac:ident ! ($($rest:tt)*)) => ($submac!(16, $($rest)*));
  (16, $submac:ident ! ($($rest:tt)*)) => ($submac!(17, $($rest)*));
  (17, $submac:ident ! ($($rest:tt)*)) => ($submac!(18, $($rest)*));
  (18, $submac:ident ! ($($rest:tt)*)) => ($submac!(19, $($rest)*));
  (19, $submac:ident ! ($($rest:tt)*)) => ($submac!(20, $($rest)*));
  (20, $submac:ident ! ($($rest:tt)*)) => ($submac!(21, $($rest)*));
  (21, $submac:ident ! ($($rest:tt)*)) => ($submac!(22, $($rest)*));
  (22, $submac:ident ! ($($rest:tt)*)) => ($submac!(23, $($rest)*));
  (23, $submac:ident ! ($($rest:tt)*)) => ($submac!(24, $($rest)*));
  (24, $submac:ident ! ($($rest:tt)*)) => ($submac!(25, $($rest)*));
  (25, $submac:ident ! ($($rest:tt)*)) => ($submac!(26, $($rest)*));
  (26, $submac:ident ! ($($rest:tt)*)) => ($submac!(27, $($rest)*));
  (27, $submac:ident ! ($($rest:tt)*)) => ($submac!(28, $($rest)*));
  (28, $submac:ident ! ($($rest:tt)*)) => ($submac!(29, $($rest)*));
  (29, $submac:ident ! ($($rest:tt)*)) => ($submac!(30, $($rest)*));
  (30, $submac:ident ! ($($rest:tt)*)) => ($submac!(31, $($rest)*));
  (31, $submac:ident ! ($($rest:tt)*)) => ($submac!(32, $($rest)*));
  (32, $submac:ident ! ($($rest:tt)*)) => ($submac!(33, $($rest)*));
  (33, $submac:ident ! ($($rest:tt)*)) => ($submac!(34, $($rest)*));
  (34, $submac:ident ! ($($rest:tt)*)) => ($submac!(35, $($rest)*));
  (35, $submac:ident ! ($($rest:tt)*)) => ($submac!(36, $($rest)*));
  (36, $submac:ident ! ($($rest:tt)*)) => ($submac!(37, $($rest)*));
  (37, $submac:ident ! ($($rest:tt)*)) => ($submac!(38, $($rest)*));
  (38, $submac:ident ! ($($rest:tt)*)) => ($submac!(39, $($rest)*));
  (39, $submac:ident ! ($($rest:tt)*)) => ($submac!(40, $($rest)*));
  (40, $submac:ident ! ($($rest:tt)*)) => ($submac!(41, $($rest)*));
);

pub(crate) trait Permutation<'a, O> {
    fn permutation(&mut self, input: &'a [u8]) -> IResult<&'a [u8], O, Error>;
}

pub(crate) fn matroska_permutation<'a, O, List: Permutation<'a, O>>(
    mut l: List,
) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], O, Error> {
    move |i| l.permutation(i)
}

macro_rules! permutation_trait(
  (
    $name1:ident $ty1:ident $item1:ident
    $name2:ident $ty2:ident $item2:ident
    $($name3:ident $ty3:ident $item3:ident)*
  ) => (
    permutation_trait!(__impl $name1 $ty1 $item1, $name2 $ty2 $item2; $($name3 $ty3 $item3)*);
  );
  (
    __impl $($name:ident $ty:ident $item:ident),+;
    $name1:ident $ty1:ident $item1:ident $($name2:ident $ty2:ident $item2:ident)*
  ) => (
    permutation_trait_impl!($($name $ty $item),+);
    permutation_trait!(__impl $($name $ty $item),+ , $name1 $ty1 $item1; $($name2 $ty2 $item2)*);
  );
  (__impl $($name:ident $ty:ident $item:ident),+;) => (
    permutation_trait_impl!($($name $ty $item),+);
  );
);

// Manual implementation for a single Parser (not covered by tuples)
impl<'a, A, FnA: Parser<&'a [u8], A, Error>> Permutation<'a, Option<A>> for FnA {
    fn permutation(&mut self, mut input: &'a [u8]) -> IResult<&'a [u8], Option<A>, Error> {
        while let Ok((i, _)) = crate::ebml::void(input) {
            input = i;
        }

        match self.parse(input) {
            Ok((i, o)) => Ok((i, Some(o))),
            Err(Err::Error(_)) => Ok((input, None)),
            Err(e) => Err(e),
        }
    }
}

macro_rules! permutation_trait_impl(
  ($($name:ident $ty:ident $item:ident),+) => (
    impl<'a,
      $($ty),+ ,
      $($name: Parser<&'a [u8], $ty, Error>),+
    > Permutation<'a, ($(Option<$ty>),+)> for ( $($name),+ ) {

      fn permutation(&mut self, mut input: &'a [u8]) -> IResult<&'a [u8], ( $(Option<$ty>),+ ), Error> {
        let mut res = ($(Option::<$ty>::None),+);
        let mut err = Error::from_error_kind(input, ErrorKind::Permutation);

        loop {
          let l = input.len();

          // Skip Void Element
          if let Ok((i, _)) = crate::ebml::void(input) {
            input = i;
          }

          try_parse!(0, self, input, res, err, $($name)+);

          // Have all parsers (including void) failed?
          if l == input.len() {
            // Skip unknown Element if possible.
            if let Error { id: _, kind: $crate::ebml::ErrorKind::MissingElement } = err {
              if let Ok((i, id)) = $crate::ebml::skip_element(input) {
                if let Some(name) = $crate::ebml::DEPRECATED.get(&id) {
                  log::warn!("Skipped deprecated Element '{name}' ({id:#0X})");
                } else {
                  log::warn!("Skipped unknown Element {id:#0X}");
                }

                input = i;
                continue;
              }
            }

            // Otherwise, end parsing.
            break;
          }
        }

        Ok((input, res))
      }
    }
  );
);

macro_rules! try_parse(
  ($it:tt, $self:expr, $input:ident, $res:expr, $err:expr, $head:ident $($id:ident)*) => (
    if $res.$it.is_none() {
      match $self.$it.parse($input) {
        Ok((i, o)) => {
          $input = i;
          $res.$it = Some(o);
        }
        Err(Err::Error(e)) => {
          $err = $err.or(e);
        }
        Err(e) => return Err(e),
      };
    }
    succ!($it, try_parse!($self, $input, $res, $err, $($id)*));
  );
  ($it:tt, $self:expr, $input:ident, $res:expr, $err:expr,) => ();
);

permutation_trait!(
  FnA A a
  FnB B b
  FnC C c
  FnD D d
  FnE E e
  FnF F f
  FnG G g
  FnH H h
  FnI I i
  FnJ J j
  FnK K k
  FnL L l
  FnM M m
  FnN N n
  FnO O o
  FnP P p
  FnQ Q q
  FnR R r
  FnS S s
  FnT T t
  FnU U u
  FnV V v
  FnW W w
  FnX X x
  FnY Y y
  FnZ Z z
  FnAA AA aa
  FnAB AB ab
  FnAC AC ac
  FnAD AD ad
  FnAE AE ae
  FnAF AF af
  FnAG AG ag
  FnAH AH ah
  FnAI AI ai
  FnAJ AJ aj
  FnAK AK ak
  FnAL AL al
  FnAM AM am
  FnAN AN an
  FnAO AO ao
);
