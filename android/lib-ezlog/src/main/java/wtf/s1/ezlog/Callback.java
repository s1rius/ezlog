package wtf.s1.ezlog;

public interface Callback {

    public void onLogsFetchSuccess(String logName, String date, String[] logs);

    public void onLogsFetchFail(String logName, String date, String err);
}
