pub(crate) trait MapPair {
    type A;
    type B;
    type Output<A, B>;

    fn map_first<C>(
        self,
        f: impl FnOnce(Self::A) -> C,
    ) -> Self::Output<C, Self::B>;
    fn map_second<C>(
        self,
        f: impl FnOnce(Self::B) -> C,
    ) -> Self::Output<Self::A, C>;
}

impl<A, B, E> MapPair for Result<(A, B), E> {
    type A = A;
    type B = B;
    type Output<First, Second> = Result<(First, Second), E>;

    fn map_first<C>(
        self,
        f: impl FnOnce(Self::A) -> C,
    ) -> Self::Output<C, Self::B> {
        match self {
            Ok((a, b)) => Ok((f(a), b)),
            Err(e) => Err(e),
        }
    }

    fn map_second<C>(
        self,
        f: impl FnOnce(Self::B) -> C,
    ) -> Self::Output<Self::A, C> {
        match self {
            Ok((a, b)) => Ok((a, f(b))),
            Err(e) => Err(e),
        }
    }
}

impl<A, B> MapPair for (A, B) {
    type A = A;
    type B = B;
    type Output<First, Second> = (First, Second);

    fn map_first<C>(
        self,
        f: impl FnOnce(Self::A) -> C,
    ) -> Self::Output<C, Self::B> {
        (f(self.0), self.1)
    }

    fn map_second<C>(
        self,
        f: impl FnOnce(Self::B) -> C,
    ) -> Self::Output<Self::A, C> {
        (self.0, f(self.1))
    }
}

impl<A, B> MapPair for Option<(A, B)> {
    type A = A;
    type B = B;
    type Output<First, Second> = Option<(First, Second)>;

    fn map_first<C>(
        self,
        f: impl FnOnce(Self::A) -> C,
    ) -> Self::Output<C, Self::B> {
        match self {
            Some((a, b)) => Some((f(a), b)),
            None => None,
        }
    }

    fn map_second<C>(
        self,
        f: impl FnOnce(Self::B) -> C,
    ) -> Self::Output<Self::A, C> {
        match self {
            Some((a, b)) => Some((a, f(b))),
            None => None,
        }
    }
}
