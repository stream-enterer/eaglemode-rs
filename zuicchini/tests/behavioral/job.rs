use zuicchini::emCore::emJob::{emJob, emJobQueue, JobState};
use zuicchini::emCore::emScheduler::EngineScheduler;

#[test]
fn enqueue_transitions_to_waiting() {
    let mut sched = EngineScheduler::new();
    let mut queue = emJobQueue::new();

    let job = emJob::new(5.0, &mut sched);
    let sig = job.GetStateSignal();
    let id = queue.EnqueueJob(job, &mut sched);

    assert_eq!(queue.GetRec(id).unwrap().GetState(), JobState::Waiting);
    assert!(!queue.IsEmpty());

    queue.AbortJob(id, &mut sched);
    sched.remove_signal(sig);
}

#[test]
fn start_transitions_to_running() {
    let mut sched = EngineScheduler::new();
    let mut queue = emJobQueue::new();

    let job = emJob::new(1.0, &mut sched);
    let sig = job.GetStateSignal();
    let id = queue.EnqueueJob(job, &mut sched);
    queue.StartJob(id, &mut sched);

    assert_eq!(queue.GetRec(id).unwrap().GetState(), JobState::Running);
    assert!(!queue.IsEmpty());

    queue.SucceedJob(id, &mut sched);
    sched.remove_signal(sig);
}

#[test]
fn succeed_transitions_to_success() {
    let mut sched = EngineScheduler::new();
    let mut queue = emJobQueue::new();

    let job = emJob::new(1.0, &mut sched);
    let sig = job.GetStateSignal();
    let id = queue.EnqueueJob(job, &mut sched);
    queue.StartJob(id, &mut sched);
    queue.SucceedJob(id, &mut sched);

    assert_eq!(queue.GetRec(id).unwrap().GetState(), JobState::Success);
    assert!(queue.IsEmpty());

    sched.remove_signal(sig);
}

#[test]
fn fail_records_error_text() {
    let mut sched = EngineScheduler::new();
    let mut queue = emJobQueue::new();

    let job = emJob::new(1.0, &mut sched);
    let sig = job.GetStateSignal();
    let id = queue.EnqueueJob(job, &mut sched);
    queue.StartJob(id, &mut sched);
    queue.FailJob(id, "out of memory".to_string(), &mut sched);

    let j = queue.GetRec(id).unwrap();
    assert_eq!(j.GetState(), JobState::Error);
    assert_eq!(j.GetErrorText(), "out of memory");
    assert!(queue.IsEmpty());

    sched.remove_signal(sig);
}

#[test]
fn abort_transitions_to_aborted() {
    let mut sched = EngineScheduler::new();
    let mut queue = emJobQueue::new();

    let job = emJob::new(1.0, &mut sched);
    let sig = job.GetStateSignal();
    let id = queue.EnqueueJob(job, &mut sched);
    queue.AbortJob(id, &mut sched);

    assert_eq!(queue.GetRec(id).unwrap().GetState(), JobState::Aborted);
    assert!(queue.IsEmpty());

    sched.remove_signal(sig);
}

#[test]
fn priority_ordering_highest_first() {
    let mut sched = EngineScheduler::new();
    let mut queue = emJobQueue::new();

    let job_low = emJob::new(1.0, &mut sched);
    let sig_low = job_low.GetStateSignal();
    let id_low = queue.EnqueueJob(job_low, &mut sched);

    let job_mid = emJob::new(5.0, &mut sched);
    let sig_mid = job_mid.GetStateSignal();
    let id_mid = queue.EnqueueJob(job_mid, &mut sched);

    let job_high = emJob::new(10.0, &mut sched);
    let sig_high = job_high.GetStateSignal();
    let id_high = queue.EnqueueJob(job_high, &mut sched);

    // StartNextJob picks highest GetPriority
    let started = queue.StartNextJob(&mut sched);
    assert_eq!(started, Some(id_high));
    queue.SucceedJob(id_high, &mut sched);

    // Next highest
    let started = queue.StartNextJob(&mut sched);
    assert_eq!(started, Some(id_mid));
    queue.SucceedJob(id_mid, &mut sched);

    // Lowest
    let started = queue.StartNextJob(&mut sched);
    assert_eq!(started, Some(id_low));
    queue.SucceedJob(id_low, &mut sched);

    assert!(queue.IsEmpty());

    sched.remove_signal(sig_low);
    sched.remove_signal(sig_mid);
    sched.remove_signal(sig_high);
}

#[test]
fn set_priority_reorders_waiting() {
    let mut sched = EngineScheduler::new();
    let mut queue = emJobQueue::new();

    let job_a = emJob::new(1.0, &mut sched);
    let sig_a = job_a.GetStateSignal();
    let id_a = queue.EnqueueJob(job_a, &mut sched);

    let job_b = emJob::new(10.0, &mut sched);
    let sig_b = job_b.GetStateSignal();
    let id_b = queue.EnqueueJob(job_b, &mut sched);

    // Boost A above B
    queue.SetPriority(id_a, 20.0);

    let started = queue.StartNextJob(&mut sched);
    assert_eq!(started, Some(id_a));

    queue.AbortJob(id_a, &mut sched);
    queue.AbortJob(id_b, &mut sched);
    sched.remove_signal(sig_a);
    sched.remove_signal(sig_b);
}

#[test]
fn fail_all_running_and_waiting() {
    let mut sched = EngineScheduler::new();
    let mut queue = emJobQueue::new();

    let job1 = emJob::new(1.0, &mut sched);
    let sig1 = job1.GetStateSignal();
    let id1 = queue.EnqueueJob(job1, &mut sched);
    queue.StartJob(id1, &mut sched);

    let job2 = emJob::new(2.0, &mut sched);
    let sig2 = job2.GetStateSignal();
    let id2 = queue.EnqueueJob(job2, &mut sched);

    let job3 = emJob::new(3.0, &mut sched);
    let sig3 = job3.GetStateSignal();
    let id3 = queue.EnqueueJob(job3, &mut sched);

    queue.FailAllJobs("shutdown", &mut sched);

    assert_eq!(queue.GetRec(id1).unwrap().GetState(), JobState::Error);
    assert_eq!(queue.GetRec(id2).unwrap().GetState(), JobState::Error);
    assert_eq!(queue.GetRec(id3).unwrap().GetState(), JobState::Error);
    assert_eq!(queue.GetRec(id1).unwrap().GetErrorText(), "shutdown");
    assert!(queue.IsEmpty());

    sched.remove_signal(sig1);
    sched.remove_signal(sig2);
    sched.remove_signal(sig3);
}

#[test]
fn waiting_and_running_job_lists() {
    let mut sched = EngineScheduler::new();
    let mut queue = emJobQueue::new();

    let job1 = emJob::new(1.0, &mut sched);
    let sig1 = job1.GetStateSignal();
    let id1 = queue.EnqueueJob(job1, &mut sched);

    let job2 = emJob::new(2.0, &mut sched);
    let sig2 = job2.GetStateSignal();
    let id2 = queue.EnqueueJob(job2, &mut sched);

    // Both waiting
    let waiting = queue.waiting_jobs();
    assert_eq!(waiting.len(), 2);

    // Start one
    queue.StartJob(id1, &mut sched);
    let running = queue.running_jobs();
    assert_eq!(running.len(), 1);
    assert_eq!(running[0], id1);

    let waiting = queue.waiting_jobs();
    assert_eq!(waiting.len(), 1);

    queue.AbortJob(id1, &mut sched);
    queue.AbortJob(id2, &mut sched);
    sched.remove_signal(sig1);
    sched.remove_signal(sig2);
}

#[test]
fn clear_aborts_all() {
    let mut sched = EngineScheduler::new();
    let mut queue = emJobQueue::new();

    let job1 = emJob::new(1.0, &mut sched);
    let sig1 = job1.GetStateSignal();
    let id1 = queue.EnqueueJob(job1, &mut sched);
    queue.StartJob(id1, &mut sched);

    let job2 = emJob::new(2.0, &mut sched);
    let sig2 = job2.GetStateSignal();
    let id2 = queue.EnqueueJob(job2, &mut sched);

    queue.Clear(&mut sched);

    assert_eq!(queue.GetRec(id1).unwrap().GetState(), JobState::Aborted);
    assert_eq!(queue.GetRec(id2).unwrap().GetState(), JobState::Aborted);
    assert!(queue.IsEmpty());

    sched.remove_signal(sig1);
    sched.remove_signal(sig2);
}

#[test]
fn job_priority_and_signal_accessors() {
    let mut sched = EngineScheduler::new();
    let job = emJob::new(7.5, &mut sched);
    assert_eq!(job.GetPriority(), 7.5);
    assert_eq!(job.GetState(), JobState::NotEnqueued);
    assert_eq!(job.GetErrorText(), "");

    let sig = job.GetStateSignal();
    job.remove_signal(&mut sched);
    // Signal was removed (no panic)
    let _ = sig;
}

#[test]
fn start_next_returns_none_when_empty() {
    let mut sched = EngineScheduler::new();
    let mut queue = emJobQueue::new();
    assert!(queue.StartNextJob(&mut sched).is_none());
}

#[test]
fn first_waiting_and_running_none_when_empty() {
    let _sched = EngineScheduler::new();
    let mut queue = emJobQueue::new();
    assert!(queue.GetFirstWaitingJob().is_none());
    assert!(queue.GetFirstRunningJob().is_none());
}
