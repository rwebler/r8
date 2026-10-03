10 REM Euclid's algorithm using a subroutine and integer division
20 PRINT "First positive integer?"
30 INPUT A
40 PRINT "Second positive integer?"
50 INPUT B
60 IF A <= 0 THEN GOTO 150
70 IF B <= 0 THEN GOTO 150
80 GOSUB 200
90 PRINT "GCD = "; A
100 END
150 PRINT "Use integers from 1 to 32767."
160 END
200 IF B = 0 THEN RETURN
210 R = A - (A / B) * B
220 A = B
230 B = R
240 GOTO 200
