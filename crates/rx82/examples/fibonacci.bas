10 REM First sixteen Fibonacci numbers, within signed 16-bit range
20 A = 0
30 B = 1
40 PRINT "N", "FIBONACCI"
50 FOR I = 0 TO 15
60 PRINT I, A
70 C = A + B
80 A = B
90 B = C
100 NEXT I
110 END
