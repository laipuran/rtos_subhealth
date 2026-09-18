import gc
import threading
import weakref

from mock_exec_layer.terminal_state import GoalTerminalCoordinator


class Goal:
    pass


def test_cancel_claim_prevents_success_commit():
    coordinator = GoalTerminalCoordinator()
    goal = Goal()

    assert coordinator.request_cancel(goal)
    assert coordinator.is_cancel_pending(goal)
    assert not coordinator.commit_success(goal)


def test_cancel_claim_prevents_abort_and_commits_canceled():
    coordinator = GoalTerminalCoordinator()
    goal = Goal()

    assert coordinator.request_cancel(goal)
    assert not coordinator.commit_abort(goal)
    assert coordinator.commit_canceled(goal)
    assert not coordinator.request_cancel(goal)


def test_success_commit_rejects_later_cancel():
    coordinator = GoalTerminalCoordinator()
    goal = Goal()

    assert coordinator.commit_success(goal)
    assert not coordinator.request_cancel(goal)


def test_terminal_claim_is_atomic_under_concurrent_requests():
    for _ in range(100):
        coordinator = GoalTerminalCoordinator()
        goal = Goal()
        barrier = threading.Barrier(3)
        outcomes = {}

        def cancel():
            barrier.wait()
            outcomes['cancel'] = coordinator.request_cancel(goal)

        def succeed():
            barrier.wait()
            outcomes['success'] = coordinator.commit_success(goal)

        cancel_thread = threading.Thread(target=cancel)
        success_thread = threading.Thread(target=succeed)
        cancel_thread.start()
        success_thread.start()
        barrier.wait()
        cancel_thread.join()
        success_thread.join()

        assert outcomes in (
            {'cancel': True, 'success': False},
            {'cancel': False, 'success': True},
        )


def test_different_goals_have_independent_terminal_states():
    coordinator = GoalTerminalCoordinator()
    canceled_goal = Goal()
    successful_goal = Goal()

    assert coordinator.request_cancel(canceled_goal)
    assert coordinator.commit_success(successful_goal)
    assert coordinator.is_cancel_pending(canceled_goal)
    assert not coordinator.is_cancel_pending(successful_goal)


def test_goal_state_does_not_outlive_goal_handle():
    coordinator = GoalTerminalCoordinator()
    goal = Goal()
    goal_reference = weakref.ref(goal)
    assert coordinator.commit_success(goal)
    assert coordinator.tracked_goal_count() == 1

    del goal
    gc.collect()

    assert goal_reference() is None
    assert coordinator.tracked_goal_count() == 0
