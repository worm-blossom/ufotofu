use crate::Consumer;

#[derive(Copy, Clone, Hash, Ord, Eq, PartialEq, PartialOrd)]
pub struct MapFinal<C, MF>
where
    C: Consumer,
{
    /// An implementer of the `Consumer` traits.
    inner: C,
    map_final: FnOnce(C::Final) -> MF,
}
