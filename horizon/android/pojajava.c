#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

typedef int (*jli_launch_t)(int, char **, int, const char **, int, const char **,
                            const char *, const char *, const char *, const char *,
                            unsigned char, unsigned char, unsigned char, int);

static void *open_lib(const char *home, const char *relative, int required) {
    char path[4096];
    snprintf(path, sizeof path, "%s/%s", home, relative);
    if (access(path, R_OK) != 0) {
        if (required) fprintf(stderr, "pojajava: %s is missing\n", path);
        return NULL;
    }
    void *handle = dlopen(path, RTLD_NOW | RTLD_GLOBAL);
    if (!handle && required) fprintf(stderr, "pojajava: %s: %s\n", path, dlerror());
    return handle;
}

static void open_beside_self(const char *name) {
    char self[4096];
    ssize_t n = readlink("/proc/self/exe", self, sizeof self - 1);
    if (n <= 0) return;
    self[n] = '\0';
    char *slash = strrchr(self, '/');
    if (!slash) return;
    *slash = '\0';
    char path[4096];
    snprintf(path, sizeof path, "%s/%s", self, name);
    if (!dlopen(path, RTLD_NOW | RTLD_GLOBAL)) fprintf(stderr, "pojajava: %s: %s\n", path, dlerror());
}

int main(int argc, char **argv) {
    open_beside_self("libc++_shared.so");
    const char *home = getenv("POJARUSH_JAVA_HOME");
    if (!home || !*home) {
        fprintf(stderr, "pojajava: POJARUSH_JAVA_HOME is not set\n");
        return 1;
    }
    void *jli = open_lib(home, "lib/jli/libjli.so", 0);
    if (!jli) jli = open_lib(home, "lib/libjli.so", 1);
    if (!jli) return 1;
    open_lib(home, "lib/server/libjvm.so", 0);
    open_lib(home, "lib/libverify.so", 0);
    open_lib(home, "lib/libjava.so", 0);
    open_lib(home, "lib/libnet.so", 0);
    open_lib(home, "lib/libnio.so", 0);
    open_lib(home, "lib/libzip.so", 0);
    jli_launch_t launch = (jli_launch_t)dlsym(jli, "JLI_Launch");
    if (!launch) {
        fprintf(stderr, "pojajava: JLI_Launch: %s\n", dlerror());
        return 1;
    }
    char exe[4096];
    snprintf(exe, sizeof exe, "%s/bin/java", home);
    argv[0] = exe;
    return launch(argc, argv, 0, NULL, 0, NULL, "25", "25", "java", "openjdk", 0, 1, 0, 0);
}
