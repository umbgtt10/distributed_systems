/// Trait for creating workers
pub trait WorkerFactory<W>: Send {
    fn create_worker(&mut self, id: usize) -> W;
}

impl<F, W> WorkerFactory<W> for F
where
    F: FnMut(usize) -> W + Send,
{
    fn create_worker(&mut self, id: usize) -> W {
        (self)(id)
    }
}
