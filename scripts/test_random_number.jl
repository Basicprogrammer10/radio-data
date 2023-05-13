using Plots, ProgressBars
include("random_lib.jl")

COUNT = 10_000
RANGE = 0.0:100.0

values = zeros(COUNT)

for i in ProgressBar(1:COUNT)
    values[i] = get_float(RANGE...)
end

display(histogram(values, title="Random Values", xlabel="Value", ylabel="Count", size=(1000, 1000)))