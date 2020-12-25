#include <stdlib.h>
#include <stdio.h>
#include "hello.h"

void say_hello(const char *name) {
    if (name != NULL) {
        printf("Hello, %s!\n", name);
    } else {
        printf("Hello!\n");
    }
}
