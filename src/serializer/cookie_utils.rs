use cookie_factory::bytes::{be_f64, be_i16};
use cookie_factory::combinator::{skip, slice};
use cookie_factory::gen::legacy_wrap;
use cookie_factory::GenError;

pub(crate) fn gen_at_offset<G>(
    offset: usize,
    f: G,
) -> impl Fn((&mut [u8], usize)) -> Result<(&mut [u8], usize), GenError>
where
    G: Fn((&mut [u8], usize)) -> Result<(&mut [u8], usize), GenError>,
{
    move |input| {
        let old_pos = input.1;
        match input.0.len() < offset {
            false => f((input.0, offset)).map(|(input, _)| (input, old_pos)),
            true => Err(GenError::BufferTooSmall(offset - input.0.len())),
        }
    }
}

pub(crate) fn gen_opt<'a, 'b, T, G: 'a, H>(
    val: Option<&'a T>,
    f: G,
) -> impl Fn((&'b mut [u8], usize)) -> Result<(&'b mut [u8], usize), GenError> + 'a
where
    G: Fn(&'a T) -> H,
    H: Fn((&'b mut [u8], usize)) -> Result<(&'b mut [u8], usize), GenError> + 'a,
{
    move |input| {
        if let Some(val) = val {
            f(val)(input)
        } else {
            Ok(input)
        }
    }
}

pub(crate) fn gen_opt_copy<'a, 'b, T: Copy + 'a, G: 'a, H>(
    val: Option<T>,
    f: G,
) -> impl Fn((&'b mut [u8], usize)) -> Result<(&'b mut [u8], usize), GenError> + 'a
where
    G: Fn(T) -> H,
    H: Fn((&'b mut [u8], usize)) -> Result<(&'b mut [u8], usize), GenError> + 'a,
{
    move |input| {
        if let Some(val) = val {
            f(val)(input)
        } else {
            Ok(input)
        }
    }
}

pub(crate) fn gen_many<'a, 'b, T: IntoIterator + Copy + 'a, G: 'a, H>(
    list: T,
    f: G,
) -> impl Fn((&'b mut [u8], usize)) -> Result<(&'b mut [u8], usize), GenError> + 'a
where
    G: Fn(<T as IntoIterator>::Item) -> H,
    H: Fn((&'b mut [u8], usize)) -> Result<(&'b mut [u8], usize), GenError> + 'a,
{
    move |input| {
        list.into_iter().fold(Ok(input), |r, v| match r {
            Err(e) => Err(e),
            Ok(x) => f(v)(x),
        })
    }
}

pub(crate) fn tuple<'a, List: tuple::Tuple<'a>>(
    l: List,
) -> impl Fn((&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError> {
    move |input| l.serialize(input)
}

#[inline]
pub(crate) fn gen_skip(
    v: usize,
) -> impl Fn((&mut [u8], usize)) -> Result<(&mut [u8], usize), GenError> {
    move |input| legacy_wrap(skip(v), input)
}

#[inline]
pub(crate) fn gen_slice<'a, 'b>(
    v: &'b [u8],
) -> impl Fn((&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError> + 'b {
    move |input| legacy_wrap(slice(v), input)
}

#[inline]
pub(crate) fn set_be_i16(x: (&mut [u8], usize), v: i16) -> Result<(&mut [u8], usize), GenError> {
    legacy_wrap(be_i16(v), x)
}

#[inline]
pub(crate) fn set_be_f64(x: (&mut [u8], usize), v: f64) -> Result<(&mut [u8], usize), GenError> {
    legacy_wrap(be_f64(v), x)
}

mod tuple {
    use cookie_factory::GenError;

    pub(crate) trait Tuple<'a> {
        fn serialize(
            &self,
            input: (&'a mut [u8], usize),
        ) -> Result<(&'a mut [u8], usize), GenError>;
    }

    macro_rules! tuple_trait(
  ($name1:ident, $name2: ident, $($name:ident),*) => (
    tuple_trait!(__impl $name1, $name2; $($name),*);
  );
  (__impl $($name:ident),+; $name1:ident, $($name2:ident),*) => (
    tuple_trait_impl!($($name),+);
    tuple_trait!(__impl $($name),+ , $name1; $($name2),*);
  );
  (__impl $($name:ident),+; $name1:ident) => (
    tuple_trait_impl!($($name),+);
    tuple_trait_impl!($($name),+, $name1);
  );
);

    macro_rules! tuple_trait_impl(
  ($($name:ident),+) => (
    impl<'a,  $($name: Fn((&'a mut[u8], usize)) -> Result<(&'a mut [u8], usize), GenError>),+> Tuple<'a> for ( $($name),+ )
      {
      fn serialize(
            &self,
            input: (&'a mut [u8], usize),
        ) -> Result<(&'a mut [u8], usize), GenError> {
        tuple_trait_inner!(0, self, input, $($name)+)
      }
    }
  );
);

    macro_rules! tuple_trait_inner(
  ($it:tt, $self:expr, $w:ident, $head:ident $($id:ident)+) => ({
    let w = $self.$it($w)?;

    succ!($it, tuple_trait_inner!($self, w, $($id)+))
  });
  ($it:tt, $self:expr, $w:ident, $head:ident) => ({
    let w = $self.$it($w)?;

    Ok(w)
  });
);

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
);

    tuple_trait!(
        FnA, FnB, FnC, FnD, FnE, FnF, FnG, FnH, FnI, FnJ, FnK, FnL, FnM, FnN, FnO, FnP, FnQ, FnR,
        FnS, FnT, FnU, FnV, FnW, FnX, FnY, FnZ, FnAA, FnAB, FnAC, FnAD, FnAE, FnAF, FnAG, FnAH,
        FnAI, FnAJ, FnAK, FnAL, FnAM, FnAN, FnAO
    );
}
