use crate::AtomicShutdownSignal;
use crate::CompletionSender;
use crate::LocalStateAccess;
use crate::MapReduceProblem;
use crate::Mapper;
use crate::Reducer;
use crate::SocketWorkChannel;
use crate::ThreadRuntime;
use crate::WordSearchProblem;

pub type MapperType = Mapper<
    WordSearchProblem,
    LocalStateAccess,
    SocketWorkChannel<<WordSearchProblem as MapReduceProblem>::MapAssignment, CompletionSender>,
    ThreadRuntime,
    AtomicShutdownSignal,
>;

pub type ReducerType = Reducer<
    WordSearchProblem,
    LocalStateAccess,
    SocketWorkChannel<<WordSearchProblem as MapReduceProblem>::ReduceAssignment, CompletionSender>,
    ThreadRuntime,
    AtomicShutdownSignal,
>;
