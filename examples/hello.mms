% Hello World for MMIX
% Prints "Hello, World!\n" using TRAP 0,1,1

        SETL    $1,0           % $1 = 0 (string base addr placeholder)
        GETA    $255,String    % $255 = address of String
        TRAP    0,1,1          % Fputs: print string at [$255]

% Compute 3 + 4 = 7
        SETL    $2,3
        SETL    $3,4
        ADD     $4,$2,$3       % $4 = 7

% Loop: count down from 5 to 0
        SETL    $10,5
Loop    SUB     $10,$10,1
        BNZ     $10,Loop

        TRAP    0,0,0          % Halt

String  BYTE    "Hello, World!\n",0
