package app.operit

import java.util.concurrent.CountDownLatch
import java.util.concurrent.FutureTask
import java.util.concurrent.RejectedExecutionException
import java.util.concurrent.TimeUnit

/** Captures JNI submissions deterministically; production worker allocation remains native. */
object OperitRuntimeNative {
    val jobs = java.util.ArrayDeque<Runnable>()
    /** Records one owner task without creating a Java thread. */
    fun scheduleRuntimeTask(task: Runnable) { jobs.addLast(task) }
    /** Records an already admitted service group. */
    fun scheduleRuntimeServices(tasks: Array<Runnable>) { tasks.forEach { jobs.addLast(it) } }
    /** Runs pending native receipts on the fixture's controlled caller. */
    fun runAll() { while (jobs.isNotEmpty()) jobs.removeFirst().run() }
}

/** Exercises client ordering, cancellation, duplicate submissions and shared scheduler lifecycle. */
fun main() {
    val serial = HostTaskExecutor(serial = true)
    val order = mutableListOf<Int>()
    repeat(100) { index -> serial.execute { order.add(index) } }
    check(OperitRuntimeNative.jobs.size == 1)
    serial.shutdown()
    OperitRuntimeNative.runAll()
    check(order == (0..99).toList())
    check(serial.awaitTermination(1, TimeUnit.SECONDS))
    try { serial.execute { error("closed task ran") }; error("closed client accepted work") }
    catch (_: RejectedExecutionException) { }

    val concurrent = HostTaskExecutor()
    val duplicate = Runnable { order.add(100) }
    concurrent.execute(duplicate)
    concurrent.execute(duplicate)
    OperitRuntimeNative.runAll()
    check(order.takeLast(2) == listOf(100, 100))
    val never = FutureTask<Unit> { error("cancelled task ran") }
    concurrent.execute(never)
    check(concurrent.shutdownNow() == mutableListOf<Runnable>(never))
    check(never.isCancelled)
    OperitRuntimeNative.runAll()
    check(concurrent.awaitTermination(1, TimeUnit.SECONDS))

    val queuedSerial = HostTaskExecutor(serial = true)
    val first = queuedSerial.submit { error("cancelled serial task ran") }
    val second = queuedSerial.submit { error("cancelled serial task ran") }
    check(queuedSerial.shutdownNow().size == 2)
    check(first.isCancelled && second.isCancelled)
    OperitRuntimeNative.runAll()
    check(queuedSerial.isTerminated)

    val services = HostTaskExecutor(serial = true, service = true)
    services.execute { order.add(101) }
    services.shutdown()
    OperitRuntimeNative.runAll()
    check(services.isTerminated)

    val interrupted = HostTaskExecutor()
    val started = CountDownLatch(1)
    val release = CountDownLatch(1)
    val active = interrupted.submit { started.countDown(); release.await() }
    val worker = Thread(OperitRuntimeNative.jobs.removeFirst())
    worker.start()
    check(started.await(1, TimeUnit.SECONDS))
    interrupted.shutdownNow()
    worker.join(1000)
    check(active.isCancelled && !worker.isAlive)
    check(interrupted.awaitTermination(1, TimeUnit.SECONDS))
    println("Host task executor contracts passed")
}
