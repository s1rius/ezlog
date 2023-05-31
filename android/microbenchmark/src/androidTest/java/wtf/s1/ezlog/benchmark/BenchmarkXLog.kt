package wtf.s1.ezlog.benchmark

import androidx.benchmark.junit4.BenchmarkRule
import androidx.benchmark.junit4.measureRepeated
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Rule
import org.junit.Test
import wtf.s1.ezlog.benchmarkable.*

class BenchmarkXLog {

    @get:Rule
    val benchmarkRule = BenchmarkRule()

    private val context = InstrumentationRegistry.getInstrumentation().targetContext

    private val log = LogController(AppXLog(context)).apply {
        this.init()
    }

    @Test
    fun benchmark_log() {
        benchmarkRule.measureRepeated {
            log.log()
        }
        log.flush()
    }
}