% Fast exponentiation (binary exponentiation) for MMIX
% Computes base^exp using repeated squaring
% Result: $0 = base^exp
%
% Algorithm:
%   result = 1
%   while exp > 0:
%     if exp is odd: result = result * base
%     base = base * base
%     exp = exp >> 1

BASE    IS      3              % base = 3
EXP     IS      13             % exponent = 13  (3^13 = 1594323)

        SETL    $0,1           % result = 1
        SETL    $1,BASE        % base
        SETL    $2,EXP         % exp

Loop    BZ      $2,Done        % if exp == 0, done
        AND     $3,$2,1        % $3 = exp & 1 (check if odd)
        BZ      $3,Skip        % if even, skip multiply
        MULU    $0,$0,$1       % result *= base
Skip    MULU    $1,$1,$1       % base *= base (square)
        SRU     $2,$2,1        % exp >>= 1
        JMP     Loop           % repeat

Done    TRAP    0,0,0          % Halt; $0 holds base^exp
