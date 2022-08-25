package wtf.s1.ezlog;

public interface EZLogCallback {

    public void onSuccess(String logName, String date, String[] logs);


    public void onFail(String logName, String date, String err);
}
