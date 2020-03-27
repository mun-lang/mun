function fibonacci(n)
    a, b, i = 0,0,0
    repeat
        if i > n then
            return a
        end
        sum = a+b
        a,b = b,sum
        i = i + 1
    until false
end

function main(n)
    return fibonacci(n)
end