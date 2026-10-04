10 REM STRINGS AND THE LENGTH OF STRINGS AND ARRAYS
20 PRINT "What is your name?"
30 INPUT N$
40 IF N$="" THEN N$="friend"
50 G$="Hello, "+N$+"!"
60 PRINT G$
70 PRINT "Greeting length: ";LEN(G$)
80 DIM A(LEN(N$))
90 FOR I=0 TO LEN(A)-1
100 A(I)=I*I
110 NEXT I
120 PRINT "Array elements: ";LEN(A)
130 PRINT "Last square: ";A(LEN(A)-1)
