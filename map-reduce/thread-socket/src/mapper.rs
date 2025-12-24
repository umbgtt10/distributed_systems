use crate::socket_completion_signaling::CompletionSender;
use crate::socket_work_channel::SocketWorkReceiver;
use map_reduce_core::map_reduce_problem::MapReduceProblem;

pub type Mapper<P, S, W, R, SD> = map_reduce_core::standard_workers::Mapper<
    P,
    S,
    W,
    R,
    SD,
    SocketWorkReceiver<<P as MapReduceProblem>::MapAssignment, CompletionSender>,
    CompletionSender,
>;
