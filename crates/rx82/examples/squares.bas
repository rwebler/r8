10 REM Print a table of squares
20 PRINT "How many squares?"
30 INPUT N
40 IF N <= 0 THEN END
50 LET I = 1
60 PRINT I, I * I
70 LET I = I + 1
80 IF I <= N THEN 60
90 END
