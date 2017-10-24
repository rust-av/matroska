#[macro_export]
macro_rules! permutation_opt (
  ($i:expr, $($rest:tt)*) => (
    {
      use ::nom::Convert;

      let mut res    = permutation_opt_init!((), $($rest)*);
      let mut input  = $i;
      let mut error  = ::std::option::Option::None;
      let mut needed = ::std::option::Option::None;

      loop {
        //println!("current res: {:?}", res);
        let mut all_done = true;

        let void_res = $crate::ebml::skip_void(input);
        if let Ok((i,_)) = void_res {
          input = i;
          continue;
        }
        permutation_opt_iterator!(0, input, all_done, needed, res, $($rest)*);

        //if we reach that part, it means none of the parsers were able to read anything
        if !all_done {
          //FIXME: should wrap the error returned by the child parser
          error = ::std::option::Option::Some(error_position!(::nom::ErrorKind::Permutation, input));
        }
        break;
      }

      if let Some(unwrapped_res) = permutation_opt_unwrap!(0, (), res, $($rest)*) {
        Ok((input, unwrapped_res))
      } else if let ::std::option::Option::Some(need) = needed {
        Err(::nom::Err::convert(need))
      } else if let ::std::option::Option::Some(e) = error {
        Err(::nom::Err::Error(e))
      } else {
        Err(::nom::Err::Error(error_position!(::nom::ErrorKind::Permutation, $i)))
      }
    }
  );
);


#[doc(hidden)]
#[macro_export]
macro_rules! permutation_opt_init (
  ((), $e:ident+, $($rest:tt)*) => (
    permutation_opt_init!((::std::vec::Vec::new()), $($rest)*)
  );
  ((), $e:ident?, $($rest:tt)*) => (
    permutation_opt_init!((::std::option::Option::None), $($rest)*)
  );
  ((), $e:ident, $($rest:tt)*) => (
    permutation_opt_init!((::std::option::Option::None), $($rest)*)
  );

  ((), $submac:ident!( $($args:tt)* )+, $($rest:tt)*) => (
    permutation_opt_init!((::std::vec::Vec::new()), $($rest)*)
  );
  ((), $submac:ident!( $($args:tt)* )?, $($rest:tt)*) => (
    permutation_opt_init!((::std::option::Option::None), $($rest)*)
  );
  ((), $submac:ident!( $($args:tt)* ), $($rest:tt)*) => (
    permutation_opt_init!((::std::option::Option::None), $($rest)*)
  );

  (($($parsed:expr),*), $e:ident+, $($rest:tt)*) => (
    permutation_opt_init!(($($parsed),* , ::std::vec::Vec::new()), $($rest)*);
  );
  (($($parsed:expr),*), $e:ident?, $($rest:tt)*) => (
    permutation_opt_init!(($($parsed),* , ::std::option::Option::None), $($rest)*);
  );
  (($($parsed:expr),*), $e:ident, $($rest:tt)*) => (
    permutation_opt_init!(($($parsed),* , ::std::option::Option::None), $($rest)*);
  );

  (($($parsed:expr),*), $submac:ident!( $($args:tt)* )+, $($rest:tt)*) => (
    permutation_opt_init!(($($parsed),* , ::std::vec::Vec::new()), $($rest)*);
  );
  (($($parsed:expr),*), $submac:ident!( $($args:tt)* )?, $($rest:tt)*) => (
    permutation_opt_init!(($($parsed),* , ::std::option::Option::None), $($rest)*);
  );
  (($($parsed:expr),*), $submac:ident!( $($args:tt)* ), $($rest:tt)*) => (
    permutation_opt_init!(($($parsed),* , ::std::option::Option::None), $($rest)*);
  );

  (($($parsed:expr),*), $e:ident+) => (
    ($($parsed),* , ::std::vec::Vec::new())
  );
  (($($parsed:expr),*), $e:ident?) => (
    ($($parsed),* , ::std::option::Option::None)
  );
  (($($parsed:expr),*), $e:ident) => (
    ($($parsed),* , ::std::option::Option::None)
  );

  (($($parsed:expr),*), $submac:ident!( $($args:tt)* )+) => (
    ($($parsed),* , ::std::vec::Vec::new())
  );
  (($($parsed:expr),*), $submac:ident!( $($args:tt)* )?) => (
    ($($parsed),* , ::std::option::Option::None)
  );
  (($($parsed:expr),*), $submac:ident!( $($args:tt)* )) => (
    ($($parsed),* , ::std::option::Option::None)
  );
  (($($parsed:expr),*),) => (
    ($($parsed),*)
  );
);

#[doc(hidden)]
#[macro_export]
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

// HACK: for some reason, Rust 1.11 does not accept $res.$it in
// permutation_opt_unwrap. This is a bit ugly, but it will have no
// impact on the generated code
#[doc(hidden)]
#[macro_export]
macro_rules! acc (
  (0, $tup:expr) => ($tup.0);
  (1, $tup:expr) => ($tup.1);
  (2, $tup:expr) => ($tup.2);
  (3, $tup:expr) => ($tup.3);
  (4, $tup:expr) => ($tup.4);
  (5, $tup:expr) => ($tup.5);
  (6, $tup:expr) => ($tup.6);
  (7, $tup:expr) => ($tup.7);
  (8, $tup:expr) => ($tup.8);
  (9, $tup:expr) => ($tup.9);
  (10, $tup:expr) => ($tup.10);
  (11, $tup:expr) => ($tup.11);
  (12, $tup:expr) => ($tup.12);
  (13, $tup:expr) => ($tup.13);
  (14, $tup:expr) => ($tup.14);
  (15, $tup:expr) => ($tup.15);
  (16, $tup:expr) => ($tup.16);
  (17, $tup:expr) => ($tup.17);
  (18, $tup:expr) => ($tup.18);
  (19, $tup:expr) => ($tup.19);
  (20, $tup:expr) => ($tup.20);
  (21, $tup:expr) => ($tup.21);
  (22, $tup:expr) => ($tup.22);
  (23, $tup:expr) => ($tup.23);
  (24, $tup:expr) => ($tup.24);
  (25, $tup:expr) => ($tup.25);
  (26, $tup:expr) => ($tup.26);
  (27, $tup:expr) => ($tup.27);
  (28, $tup:expr) => ($tup.28);
  (29, $tup:expr) => ($tup.29);
  (30, $tup:expr) => ($tup.30);
  (31, $tup:expr) => ($tup.31);
  (32, $tup:expr) => ($tup.32);
  (33, $tup:expr) => ($tup.33);
  (34, $tup:expr) => ($tup.34);
  (35, $tup:expr) => ($tup.35);
  (36, $tup:expr) => ($tup.36);
  (37, $tup:expr) => ($tup.37);
  (38, $tup:expr) => ($tup.38);
  (39, $tup:expr) => ($tup.39);
  (40, $tup:expr) => ($tup.40);
  (41, $tup:expr) => ($tup.41);
);

#[doc(hidden)]
#[macro_export]
macro_rules! permutation_opt_unwrap (
  ($it:tt,  (), $res:ident, $submac:ident!( $($args:tt)* )+, $($rest:tt)*) => (
    succ!($it, permutation_opt_unwrap!((acc!($it, $res)), $res, $($rest)*));
  );
  ($it:tt,  (), $res:ident, $submac:ident!( $($args:tt)* )?, $($rest:tt)*) => (
    succ!($it, permutation_opt_unwrap!((acc!($it, $res)), $res, $($rest)*));
  );
  ($it:tt,  (), $res:ident, $submac:ident!( $($args:tt)* ), $($rest:tt)*) => ({
    let res = acc!($it, $res);
    if res.is_some() {
      succ!($it, permutation_opt_unwrap!((res.unwrap()), $res, $($rest)*))
    } else {
      None
    }
  });

  ($it:tt, ($($parsed:expr),*), $res:ident, $e:ident+, $($rest:tt)*) => (
    permutation_opt_unwrap!($it, ($($parsed),* ), $res, call!($e)+, $($rest)*);
  );
  ($it:tt, ($($parsed:expr),*), $res:ident, $e:ident?, $($rest:tt)*) => (
    succ!($it, permutation_opt_unwrap!(($($parsed),* , acc!($it, $res)), $res, $($rest)*));
  );
  ($it:tt, ($($parsed:expr),*), $res:ident, $e:ident, $($rest:tt)*) => ({
    let res = acc!($it, $res);
    if res.is_some() {
      succ!($it, permutation_opt_unwrap!(($($parsed),* , res.unwrap()), $res, $($rest)*))
    } else {
      None
    }
  });

  ($it:tt, ($($parsed:expr),*), $res:ident, $submac:ident!( $($args:tt)* )+, $($rest:tt)*) => (
    succ!($it, permutation_opt_unwrap!(($($parsed),* , acc!($it, $res)), $res, $($rest)*));
  );
  ($it:tt, ($($parsed:expr),*), $res:ident, $submac:ident!( $($args:tt)* )?, $($rest:tt)*) => (
    succ!($it, permutation_opt_unwrap!(($($parsed),* , acc!($it, $res)), $res, $($rest)*));
  );
  ($it:tt, ($($parsed:expr),*), $res:ident, $submac:ident!( $($args:tt)* ), $($rest:tt)*) => ({
    let res = acc!($it, $res);
    if res.is_some() {
      succ!($it, permutation_opt_unwrap!(($($parsed),* , res.unwrap()), $res, $($rest)*))
    } else {
      None
    }
  });

  ($it:tt, ($($parsed:expr),*), $res:ident, $e:ident+) => (
    Some(($($parsed),* , acc!($it, $res)))
  );
  ($it:tt, ($($parsed:expr),*), $res:ident, $e:ident?) => (
    Some(($($parsed),* , { acc!($it, $res) }))
  );
  ($it:tt, ($($parsed:expr),*), $res:ident, $e:ident) => ({
    let res = acc!($it, $res);
    if res.is_some() {
      Some(($($parsed),* , { res.unwrap() }))
    } else {
      None
    }
  });


  ($it:tt, ($($parsed:expr),*), $res:ident, $submac:ident!( $($args:tt)* )+) => (
    Some(($($parsed),* , acc!($it, $res) ))
  );
  ($it:tt, ($($parsed:expr),*), $res:ident, $submac:ident!( $($args:tt)* )?) => (
    Some(($($parsed),* , acc!($it, $res) ))
  );
  ($it:tt, ($($parsed:expr),*), $res:ident, $submac:ident!( $($args:tt)* )) => ({
    let res = acc!($it, $res);
    if res.is_some() {
      Some(($($parsed),* , res.unwrap() ))
    } else {
      None
    }
  });
);

#[doc(hidden)]
#[macro_export]
macro_rules! permutation_opt_iterator (
  ($it:tt,$i:expr, $all_done:expr, $needed:expr, $res:expr, $e:ident+, $($rest:tt)*) => ({
    permutation_opt_iterator!($it, $i, $all_done, $needed, $res, call!($e)+, $($rest)*);
  });
  ($it:tt,$i:expr, $all_done:expr, $needed:expr, $res:expr, $e:ident?, $($rest:tt)*) => ({
    permutation_opt_iterator!($it, $i, $all_done, $needed, $res, call!($e), $($rest)*);
  });
  ($it:tt,$i:expr, $all_done:expr, $needed:expr, $res:expr, $e:ident, $($rest:tt)*) => ({
    permutation_opt_iterator!($it, $i, $all_done, $needed, $res, call!($e), $($rest)*);
  });

  ($it:tt, $i:expr, $all_done:expr, $needed:expr, $res:expr, $submac:ident!( $($args:tt)* )+, $($rest:tt)*) => {
    let res = &mut acc!($it, $res);
    match complete!($i, $submac!($($args)*)) {
      Ok((i,o))     => {
        $i = i;
        res.push(o);
        continue;
      },
      Err(::nom::Err::Error(_)) => {
        $all_done = false;
      },
      Err(e) => {
        $needed = ::std::option::Option::Some(e);
        break;
      }
    };
    succ!($it, permutation_opt_iterator!($i, $all_done, $needed, $res, $($rest)*));
  };
  ($it:tt, $i:expr, $all_done:expr, $needed:expr, $res:expr, $submac:ident!( $($args:tt)* )?, $($rest:tt)*) => {
    permutation_opt_iterator!($it, $i, $all_done, $needed, $res, $submac!($($args)*), $($rest)*)
  };
  ($it:tt, $i:expr, $all_done:expr, $needed:expr, $res:expr, $submac:ident!( $($args:tt)* ), $($rest:tt)*) => {
    if acc!($it, $res) == ::std::option::Option::None {
      match complete!($i, $submac!($($args)*)) {
        Ok((i,o))     => {
          $i = i;
          acc!($it, $res) = ::std::option::Option::Some(o);
          continue;
        },
        Err(::nom::Err::Error(_)) => {
          $all_done = false;
        },
        Err(e) => {
          $needed = ::std::option::Option::Some(e);
          break;
        }
      };
    }
    succ!($it, permutation_opt_iterator!($i, $all_done, $needed, $res, $($rest)*));
  };

  ($it:tt,$i:expr, $all_done:expr, $needed:expr, $res:expr, $e:ident+) => ({
    permutation_opt_iterator!($it, $i, $all_done, $needed, $res, call!($e));
  });
  ($it:tt,$i:expr, $all_done:expr, $needed:expr, $res:expr, $e:ident?) => ({
    permutation_opt_iterator!($it, $i, $all_done, $needed, $res, call!($e));
  });
  ($it:tt,$i:expr, $all_done:expr, $needed:expr, $res:expr, $e:ident) => ({
    permutation_opt_iterator!($it, $i, $all_done, $needed, $res, call!($e));
  });

  ($it:tt, $i:expr, $all_done:expr, $needed:expr, $res:expr, $submac:ident!( $($args:tt)* )+) => {
    let res = &mut acc!($it, $res);
    match complete!($i, $submac!($($args)*)) {
      Ok((i,o))     => {
        $i = i;
        res.push(o);
        continue;
      },
      Err(::nom::Err::Error(_)) => {
        $all_done = false;
      },
      Err(e) => {
        $needed = ::std::option::Option::Some(e);
        break;
      }
    };
  };
  ($it:tt, $i:expr, $all_done:expr, $needed:expr, $res:expr, $submac:ident!( $($args:tt)* )?) => {
    permutation_opt_iterator!($it, $i, $all_done, $needed, $res, $submac!($($args)*));
  };
  ($it:tt, $i:expr, $all_done:expr, $needed:expr, $res:expr, $submac:ident!( $($args:tt)* )) => {
    if acc!($it, $res) == ::std::option::Option::None {
      match complete!($i, $submac!($($args)*)) {
        Ok((i,o))     => {
          $i = i;
          acc!($it, $res) = ::std::option::Option::Some(o);
          continue;
        },
        Err(::nom::Err::Error(_)) => {
          $all_done = false;
        },
        Err(e) => {
          $needed = ::std::option::Option::Some(e);
          break;
        }
      };
    }
  };
);

#[cfg(test)]
mod tests {
    use nom::{Err,Needed};

    // reproduce the tag and take macros, because of module import order
    macro_rules! tag (
    ($i:expr, $inp: expr) => (
      {
        #[inline(always)]
        fn as_bytes<T: ::nom::AsBytes>(b: &T) -> &[u8] {
          b.as_bytes()
        }

        let expected = $inp;
        let bytes    = as_bytes(&expected);

        tag_bytes!($i,bytes)
      }
    );
  );

    macro_rules! tag_bytes (
    ($i:expr, $bytes: expr) => (
      {
        use std::cmp::min;
        let len = $i.len();
        let blen = $bytes.len();
        let m   = min(len, blen);
        let reduced = &$i[..m];
        let b       = &$bytes[..m];

        let res: ::nom::IResult<_,_> = if reduced != b {
          Err(Err::Error(error_position!(::nom::ErrorKind::Tag, $i)))
        } else if m < blen {
          Err(Err::Incomplete(::nom::Needed::Size(blen)))
        } else {
          Ok((&$i[blen..], reduced))
        };
        res
      }
    );
  );

    macro_rules! take(
    ($i:expr, $count:expr) => (
      {
        let cnt = $count as usize;
        let res:::IResult<&[u8],&[u8]> = if $i.len() < cnt {
          Err(Err::Incomplete(::nom::Needed::Size(cnt)))
        } else {
          Ok((&$i[cnt..],&$i[0..cnt]))
        };
        res
      }
    );
  );

    #[test]
    fn permutation() {
        named!(perm<(&[u8], &[u8], &[u8])>,
      permutation!(tag!("abcd"), tag!("efg"), tag!("hi"))
    );

        let expected = (&b"abcd"[..], &b"efg"[..], &b"hi"[..]);

        let a = &b"abcdefghijk"[..];
        assert_eq!(perm(a), Ok((&b"jk"[..], expected)));
        let b = &b"efgabcdhijk"[..];
        assert_eq!(perm(b), Ok((&b"jk"[..], expected)));
        let c = &b"hiefgabcdjk"[..];
        assert_eq!(perm(c), Ok((&b"jk"[..], expected)));

        let d = &b"efgxyzabcdefghi"[..];
        assert_eq!(perm(d), Err(Err::Error(error_position!(ErrorKind::Permutation, &b"xyzabcdefghi"[..]))));

        let e = &b"efgabc"[..];
        assert_eq!(perm(e), Err(Err::Incomplete(Needed::Size(7))));
    }

    #[test]
    fn optional_permutation() {
        named!(perm<(&[u8], Option<&[u8]>, &[u8], Option<&[u8]>)>,
      permutation_opt!(dbg_dmp!(tag!("abcd")), dbg_dmp!(tag!("efg"))?, dbg_dmp!(tag!("hi")), dbg_dmp!(tag!("jkl"))?)
    );

        let expected1 = (&b"abcd"[..], Some(&b"efg"[..]), &b"hi"[..], Some(&b"jkl"[..]));
        let expected2 = (&b"abcd"[..], None, &b"hi"[..], Some(&b"jkl"[..]));
        let expected3 = (&b"abcd"[..], None, &b"hi"[..], None);

        let a = &b"abcdefghijklm"[..];
        assert_eq!(perm(a), Ok((&b"m"[..], expected1)));
        let b = &b"efgabcdhijklm"[..];
        assert_eq!(perm(b), Ok((&b"m"[..], expected1)));
        let c = &b"hiefgabcdjklm"[..];
        assert_eq!(perm(c), Ok((&b"m"[..], expected1)));

        let d = &b"abcdjklhim"[..];
        assert_eq!(perm(d), Ok((&b"m"[..], expected2)));
        let e = &b"abcdhijklm"[..];
        assert_eq!(perm(e), Ok((&b"m"[..], expected2)));

        let f = &b"abcdhim"[..];
        assert_eq!(perm(f), Ok((&b"m"[..], expected3)));
        let g = &b"hiabcdm"[..];
        assert_eq!(perm(g), Ok((&b"m"[..], expected3)));
        /*
    let d = &b"efgxyzabcdefghi"[..];
    assert_eq!(perm(d), Error(error_position!(ErrorKind::Permutation, &b"xyzabcdefghi"[..])));

    let e = &b"efgabc"[..];
    assert_eq!(perm(e), Incomplete(Needed::Size(7)));
*/
    }
}
