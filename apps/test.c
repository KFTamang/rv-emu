#include <stdio.h>

void fizzbuzz(int);

int main(void)
{
    fizzbuzz(3);
    return 0;
}

void fizzbuzz(int max)
{
    char answer[500];
    int i = 0;
    while (i < max)
    {
        i++;
        if (i % 3 == 0 && i % 5 == 0)
        {
            answer[i] = '*';
            continue;
        }
        if (i % 3 == 0)
        {
            answer[i] = 'F';
            continue;
        }
        if (i % 5 == 0)
        {
            answer[i] = 'B';
            continue;
        }
        answer[i] = i;
    }
}
