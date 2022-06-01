extern void c_log(
    char *c_log_name, 
    char c_level, 
    char *c_target, 
    char *c_content);

int main(void) {
    c_log("asdf", 1, "target", "content");
    return 0;
}