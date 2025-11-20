#include <unistd.h>

int fibbonacci(int n) {
  if (n == 0)
    return 0;
  else if (n == 1)
    return 1;
  else
    return (fibbonacci(n - 1) + fibbonacci(n - 2));
}

int main() {
  for (int i = 0; i < 10; i++) {
    int res = fibbonacci(i);
    // convert integer to string
    char buf[20];
    int len = 0;
    if (res == 0) {
      buf[len++] = '0';
    } else {
      int temp = res;
      char rev[20];
      int rev_len = 0;
      while (temp > 0) {
        rev[rev_len++] = (temp % 10) + '0';
        temp /= 10;
      }
      for (int j = rev_len - 1; j >= 0; j--) {
        buf[len++] = rev[j];
      }
    }
    buf[len++] = '\n';
    write(1, buf, len);
  }
}
