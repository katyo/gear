#include <stdlib.h>
#include <stdio.h>

#include "common.h"
#include "hello.h"
#include "bye.h"

const char *program = PROGRAM;
const unsigned char version[3] = VERSION;

int main(int argc, char *argv[]) {
    if (argc > 1) {
        say_hello(argv[1]);
    } else {
        printf("%s %u.%u.%u\n", program, version[0], version[1], version[2]);
        return 1;
    }
    printf("...\n");
    say_goodbye();
    return 0;
}
