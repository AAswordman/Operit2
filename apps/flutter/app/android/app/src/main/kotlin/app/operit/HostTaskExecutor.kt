package app.operit

import android.util.Log
import java.util.ArrayDeque
import java.util.concurrent.AbstractExecutorService
import java.util.concurrent.Future
import java.util.concurrent.RejectedExecutionException
import java.util.concurrent.TimeUnit

/** Submits Java work to the same native Host scheduler that owns Rust execution. */
object HostTaskScheduler {
    /** Uses native blocking capacity without creating a Java executor pool. */
    fun execute(task: Runnable) = OperitRuntimeNative.scheduleRuntimeTask(task)

    /** Reserves an entire blocking pipeline before starting any of its components. */
    fun executeServices(tasks: Array<Runnable>) = OperitRuntimeNative.scheduleRuntimeServices(tasks)
}

/** Owns ordering and lifecycle for one client while the Host owns every worker thread. */
class HostTaskExecutor(private val serial: Boolean = false, private val service: Boolean = false) : AbstractExecutorService() {
    private val lock = Object()
    private class ClientTask(val command: Runnable)
    private val pending = ArrayDeque<ClientTask>()
    private val active = mutableSetOf<ClientTask>()
    private val accepted = mutableSetOf<ClientTask>()
    private var stopped = false
    private var draining = false
    private var submitted = 0

    /** Enqueues one task and preserves serial client order independently of other clients. */
    override fun execute(command: Runnable) {
        val entry = ClientTask(command)
        synchronized(lock) {
            if (stopped) throw RejectedExecutionException("Host task executor is closed")
            if (serial) {
                if (pending.size >= 4096) throw RejectedExecutionException("Host client queue is full")
                pending.addLast(entry)
                if (!draining) {
                    draining = true
                    try { dispatch(Runnable { drain() }) }
                    catch (error: Throwable) {
                        draining = false
                        pending.removeLast()
                        throw error
                    }
                }
            } else {
                accepted.add(entry)
                submitted++
                try {
                    dispatch(Runnable {
                        try {
                            val claimed = synchronized(lock) {
                                if (accepted.remove(entry)) { active.add(entry); true } else false
                            }
                            if (claimed) runTask(entry)
                        }
                        finally { synchronized(lock) { submitted--; lock.notifyAll() } }
                    })
                } catch (error: Throwable) {
                    submitted--
                    accepted.remove(entry)
                    throw error
                }
            }
        }
    }

    /** Selects Host capacity explicitly from the client's blocking requirements. */
    private fun dispatch(task: Runnable) {
        if (service) HostTaskScheduler.executeServices(arrayOf(task))
        else HostTaskScheduler.execute(task)
    }

    /** Drains a client's queue on allocated Host capacity without retaining an idle worker. */
    private fun drain() {
        while (true) {
            val task = synchronized(lock) {
                if (pending.isEmpty()) {
                    draining = false
                    lock.notifyAll()
                    return
                }
                pending.removeFirst().also { active.add(it) }
            }
            runTask(task)
        }
    }

    /** Contains task failure and clears Java interruption before reusing a Host worker. */
    private fun runTask(task: ClientTask) {
        Thread.interrupted()
        try { task.command.run() }
        catch (error: Throwable) { Log.e("HostTaskExecutor", "Host task failed", error) }
        finally {
            Thread.interrupted()
            synchronized(lock) { active.remove(task); lock.notifyAll() }
        }
    }

    /** Rejects later submissions while accepted tasks finish in order. */
    override fun shutdown() { synchronized(lock) { stopped = true; lock.notifyAll() } }

    /** Cancels queued futures and interrupts only futures that still own their running task. */
    override fun shutdownNow(): MutableList<Runnable> = synchronized(lock) {
        stopped = true
        val removed = (pending.toList() + accepted.toList()).toMutableList()
        pending.clear()
        accepted.clear()
        for (task in removed) if (task.command is Future<*>) task.command.cancel(false)
        for (task in active) if (task.command is Future<*>) task.command.cancel(true)
        lock.notifyAll()
        removed.map { it.command }.toMutableList()
    }

    /** Reports whether this client has stopped accepting tasks. */
    override fun isShutdown(): Boolean = synchronized(lock) { stopped }

    /** Reports whether every accepted client task has finished. */
    override fun isTerminated(): Boolean = synchronized(lock) {
        stopped && !draining && submitted == 0 && active.isEmpty()
    }

    /** Waits for client completion without changing the lifecycle of the shared Host pools. */
    override fun awaitTermination(timeout: Long, unit: TimeUnit): Boolean {
        val deadline = System.nanoTime() + unit.toNanos(timeout)
        synchronized(lock) {
            while (!isTerminated()) {
                val remaining = deadline - System.nanoTime()
                if (remaining <= 0) return false
                TimeUnit.NANOSECONDS.timedWait(lock, remaining)
            }
            return true
        }
    }
}
