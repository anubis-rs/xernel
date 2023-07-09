#pragma once

char heap[1000000];

void *malloc(unsigned long size) {
    // Implement your custom allocation logic here with alignment to 16 bytes
    // For simplicity, we'll use a static array as the heap
    static int heapPointer = 0;
    void *ptr = &heap[heapPointer];
    heapPointer += size;
    heapPointer += 16 - (heapPointer % 16);
    return ptr;
}

extern "C" void* operator new(unsigned long size) {
    // Implement your custom allocation logic here
    // For simplicity, we'll use malloc to allocate memory
    return malloc(size);
}
