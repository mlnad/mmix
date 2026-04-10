% Fibonacci sequence for MMIX
% Computes the first N Fibonacci numbers: F(0)=0, F(1)=1, F(n)=F(n-1)+F(n-2)
% Result is stored in $3 after each iteration

N       IS      10             % number of Fibonacci numbers to compute

        SETL    $0,0           % F(n-2) = 0
        SETL    $1,1           % F(n-1) = 1
        SETL    $2,N           % loop counter
        SUB     $2,$2,2        % already have F(0) and F(1), need N-2 more

Loop    ADD     $3,$0,$1       % F(n) = F(n-2) + F(n-1)
        SET     $0,$1          % F(n-2) = old F(n-1)
        SET     $1,$3          % F(n-1) = F(n)
        SUB     $2,$2,1        % counter--
        BNZ     $2,Loop        % repeat if counter != 0

        TRAP    0,0,0          % Halt; $3 holds F(N-1)
