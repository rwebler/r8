10 REM Factorials through 7! fit in signed 16-bit integers
20 PRINT "N", "FACTORIAL"
30 FOR N = 0 TO 7
40 GOSUB 100
50 PRINT N, F
60 NEXT N
70 END
100 REM An empty loop makes 0! equal to one
110 F = 1
120 FOR I = 1 TO N
130 F = F * I
140 NEXT I
150 RETURN
