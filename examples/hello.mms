% Hello World for MMIX
% Prints "Hello, World!\n" using TRAP 0,1,1

Main    SETL    $1,0           % $1 = 0 (string base addr placeholder)
        GETA    $255,String    % $255 = address of String
        TRAP    0,1,1          % Fputs: print string at [$255]
        TRAP    0,0,0          % Halt

String  BYTE    "Hello, World!\n",0
