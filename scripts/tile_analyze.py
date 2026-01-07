
import csv
import matplotlib.pyplot as plt
import numpy as np

# melbourne takes 270 seconds with 16 cores on a Turin machine (round up to 270s)
# which works out to about 12s/angle. If we want the number of CPU seconds for the entire
# computation, then we need 270s * 16 cores, which gets a total CPU time of ~1.2 hours or 4320 seconds
CPU_SECONDS_PER_POINT = 4320 / (4060.0 ** 2.0)
CORE_COUNT = 48

COST_PER_CORE_DAY = 0.04 * CORE_COUNT * 24

if __name__ == "__main__":
    max_los = []
    with open("../website/public/tiles.csv") as f:
        reader = csv.reader(f, delimiter=",")

        for row in reader:
            max_los.append(float(row[2]))

    max_los = sorted(map(lambda x: x / 100.0, max_los))

    total_area = sum(map(lambda x: x*x, max_los))

    median_area = total_area / 2
    area = 0.0
    for (i, tile) in enumerate(max_los):
        area += tile*tile
        if area > median_area:
            print(f"must process all tiles up to {tile}kms to get 50% of the world which is {i+1} tiles")
            break



    middle = len(max_los) // 2

    secs = 0.0
    for kilometers in max_los:
        secs += CPU_SECONDS_PER_POINT * (kilometers ** 2.0)

    num_days = secs / 60 / 60 / 24 / CORE_COUNT
    total_cost = COST_PER_CORE_DAY * num_days
    print(f"It will take {num_days} days to compute on {CORE_COUNT} cores and cost ${total_cost}")

    fig, ax = plt.subplots()

    median_value = np.median(max_los)

    ax.hist(max_los, bins=20, edgecolor='black')

    # 4. Add a vertical line at the median
    ax.axvline(median_value, color='red', linestyle='dashed', linewidth=2, label=f'Median: {median_value}')
    ax.legend()

    plt.savefig("../data.png")

