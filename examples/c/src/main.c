#include <stdlib.h>
#include <stdio.h>

#include "hello.h"
#include "bye.h"

int main(int argc, char argv[]) {
    if (argc > 1) {
        say_hello(argv[1]);
    } else {
        say_hello(NULL);
    }
    printf("...\n");
    say_goodbye();
    return 0;
}
