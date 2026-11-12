// pmem: one line per readable process: pid ppid responsible_pid phys_footprint_bytes cpu_ns name
// cpu_ns is user+system from proc_pid_rusage, converted from Mach ticks.
#include <libproc.h>
#include <mach/mach_time.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/resource.h>
extern pid_t responsibility_get_pid_responsible_for_pid(pid_t) __attribute__((weak_import));
int main(void) {
  mach_timebase_info_data_t tb; mach_timebase_info(&tb);
  int n = proc_listallpids(NULL, 0); pid_t *pids = calloc(n * 2, sizeof(pid_t));
  n = proc_listallpids(pids, n * 2 * sizeof(pid_t));
  for (int i = 0; i < n; i++) {
    struct rusage_info_v4 ru; struct proc_bsdinfo bi; char name[2 * MAXCOMLEN + 1] = {0};
    if (proc_pid_rusage(pids[i], RUSAGE_INFO_V4, (rusage_info_t *)&ru) != 0) continue;
    if (proc_pidinfo(pids[i], PROC_PIDTBSDINFO, 0, &bi, sizeof bi) != sizeof bi) continue;
    proc_name(pids[i], name, sizeof name);
    pid_t resp = responsibility_get_pid_responsible_for_pid ? responsibility_get_pid_responsible_for_pid(pids[i]) : -1;
    unsigned long long cpu = (ru.ri_user_time + ru.ri_system_time) * tb.numer / tb.denom;
    printf("%d\t%d\t%d\t%llu\t%llu\t%s\n", pids[i], bi.pbi_ppid, resp, ru.ri_phys_footprint, cpu, name);
  }
  return 0;
}
