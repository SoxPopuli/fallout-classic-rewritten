pub(crate) trait MapPair<RetFirst, RetSecond> {
    type A;
    type B;
    type O;
    fn map_first(self, f: impl FnOnce(Self::A) -> Self::O) -> RetFirst;

    fn map_second(self, f: impl FnOnce(Self::B) -> Self::O) -> RetSecond;
}

impl<A, B, O, E> MapPair<Result<(O, B), E>, Result<(A, O), E>>
    for Result<(A, B), E>
{
    type A = A;
    type B = B;
    type O = O;

    fn map_first(self, f: impl FnOnce(Self::A) -> O) -> Result<(O, B), E> {
        match self {
            Ok((a, b)) => Ok((f(a), b)),
            Err(e) => Err(e),
        }
    }

    fn map_second(
        self,
        f: impl FnOnce(Self::B) -> Self::O,
    ) -> Result<(A, O), E> {
        match self {
            Ok((a, b)) => Ok((a, f(b))),
            Err(e) => Err(e),
        }
    }
}

impl<A, B, O> MapPair<Option<(O, B)>, Option<(A, O)>> for Option<(A, B)> {
    type A = A;
    type B = B;
    type O = O;

    fn map_first(self, f: impl FnOnce(Self::A) -> Self::O) -> Option<(O, B)> {
        match self {
            Some((a, b)) => Some((f(a), b)),
            None => None,
        }
    }

    fn map_second(self, f: impl FnOnce(Self::B) -> Self::O) -> Option<(A, O)> {
        match self {
            Some((a, b)) => Some((a, f(b))),
            None => None,
        }
    }
}

impl<A, B, O> MapPair<(O, B), (A, O)> for (A, B) {
    type A = A;
    type B = B;
    type O = O;

    fn map_first(self, f: impl FnOnce(Self::A) -> Self::O) -> (O, B) {
        (f(self.0), self.1)
    }

    fn map_second(self, f: impl FnOnce(Self::B) -> Self::O) -> (A, O) {
        (self.0, f(self.1))
    }
}
