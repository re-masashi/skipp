#include <stdio.h>
#include <stdbool.h>
#include <gc.h>

extern "C" {
    struct Person {
        int age;
        bool alive;
    };

    void print_person(struct Person person) { // Add 'struct' keyword before Person
        printf("alive person? %d\n", person.alive); // Access struct member 'alive' directly
        printf("Age: %d %d\n", person.age, person.alive); // Access struct members 'age' and 'alive'
    }

    struct Person create_person(int age, bool alive) { // Add 'struct' keyword before Person
        struct Person p = {age, alive}; // Initialize struct Person
        print_person(p); // Call function to print person
        return p; // Return the created person
    }

    int println(int n) {
        printf("%d\n", n); // Print integer followed by a newline
        return n; // Return the integer
    }

    int index_arr(int arr[], int i) {
        return arr[i]; // Return the value at index i in the array arr
    }
}