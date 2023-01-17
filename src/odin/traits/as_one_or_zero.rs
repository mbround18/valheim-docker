pub trait AsOneOrZero
where
  Self: PartialEq<bool>,
{
  #[allow(clippy::wrong_self_convention)]
  fn as_string(self) -> String
  where
    Self: Sized,
  {
    if self == true { "1" } else { "0" }.to_string()
  }
}

impl AsOneOrZero for bool {}

#[cfg(test)]
mod as_one_or_zero_test {
  use crate::traits::AsOneOrZero;

  #[test]
  fn returns_one() {
    debug_assert!({ true.as_string().eq("1") })
  }

  #[test]
  fn returns_zero() {
    debug_assert!({ false.as_string().eq("0") })
  }
}
