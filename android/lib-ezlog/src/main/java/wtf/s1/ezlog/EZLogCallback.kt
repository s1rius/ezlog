package wtf.s1.ezlog

interface EZLogCallback {
    fun onSuccess(logName: String?, date: String?, logs: Array<String?>?)
    fun onFail(logName: String?, date: String?, err: String?)
}