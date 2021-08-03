#define KBUILD_MODNAME "bytecode"
#include <linux/kconfig.h>
#include <linux/bpf.h>
#include <linux/tcp.h>
#include <linux/version.h>
#include "bpf_helpers.h"

#ifndef LINUX_VERSION_CODE
#pragma message("LINUX_VERSION_CODE not defined")
#endif

#define bpf_printk(fmt, ...)                             \
({                                                       \
    char _fmt[] = fmt;                                   \
    bpf_trace_printk(_fmt, sizeof(_fmt), ##__VA_ARGS__); \
})

static unsigned long long (*bpf_get_current_task)(void) =
	(void *) BPF_FUNC_get_current_task;

static int (*bpf_probe_read_str)(void *ctx, __u32 size, const void *unsafe_ptr) =
	(void *) BPF_FUNC_probe_read_str;

struct EBPF_bpf_map_def {
	unsigned int type;
	unsigned int key_size;
	unsigned int value_size;
	unsigned int max_entries;
	unsigned int map_flags;
    unsigned int inner_map_fd;
    unsigned int numa_node;
    uint8_t      map_name[16];
    unsigned int map_ifindex;
};

struct event {
    u32 event;
    u32 pid;
    u32 proto;
    u32 saddr;
    u32 sport;
    u32 daddr;
    u32 dport;
};

SEC("maps/events")
struct EBPF_bpf_map_def events = {
    .type        = BPF_MAP_TYPE_PERF_EVENT_ARRAY,
    .key_size    = sizeof(int),
    .value_size  = sizeof(int),
    .max_entries = 512,
};

SEC("maps/socks")
struct EBPF_bpf_map_def socks = {
    .type        = BPF_MAP_TYPE_HASH,
    .key_size    = sizeof(u32),
    .value_size  = sizeof(struct sock *),
    .max_entries = 512,
};

SEC("maps/procs")
struct EBPF_bpf_map_def procs = {
    .type        = BPF_MAP_TYPE_LRU_HASH,
    .key_size    = sizeof(struct sock *),
    .value_size  = sizeof(u64),
    .max_entries = 512,
};

SEC("kprobe/call-tcp-connect")
int bpf_call_tcp_connect(struct pt_regs *ctx) {
    struct sock *sk = (void *) PT_REGS_PARM1(ctx);

    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    u32 tid = pid_tgid;

    bpf_map_update_elem(&socks, &tid, &sk, 0);

    return 0;
}

SEC("kretprobe/exit-tcp-connect")
int bpf_exit_tcp_connect(struct pt_regs *ctx) {
    int rc = PT_REGS_RC(ctx);

    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    u32 tid = pid_tgid;

    struct sock **skp = bpf_map_lookup_elem(&socks, &tid);
    if (skp == 0) {
        return 0;
    }

    if (rc != 0) {
        bpf_map_delete_elem(&socks, &tid);
        return 0;
    }

    struct sock *sk = *skp;
    struct sock_common sc;
    bpf_probe_read(&sc, sizeof(sc), &sk->__sk_common);

    struct event event = {
        .event = 1,
        .pid   = pid,
        .proto = 6,
        .saddr = sc.skc_rcv_saddr,
        .sport = sc.skc_num,
        .daddr = sc.skc_daddr,
        .dport = ntohs(sc.skc_dport),
    };

    rc = bpf_perf_event_output(ctx, &events, BPF_F_CURRENT_CPU, &event, sizeof(event));
    if (rc != 0) {
        bpf_printk("perf event output failure: %d\n", rc);
    }
    bpf_map_delete_elem(&socks, &tid);


    return 0;
}

SEC("kretprobe/inet_csk_accept")
int bpf_call_inet_csk_accept(struct pt_regs *ctx) {
    struct sock *sk = (void *) PT_REGS_RC(ctx);

    if (sk == NULL) {
        return 0;
    }

    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    u32 tid = pid_tgid;

    struct sock_common sc;
    bpf_probe_read(&sc, sizeof(sc), &sk->__sk_common);

    struct event event = {
        .event = 2,
        .pid   = pid,
        .proto = 6,
        .saddr = sc.skc_rcv_saddr,
        .sport = sc.skc_num,
        .daddr = sc.skc_daddr,
        .dport = ntohs(sc.skc_dport),
    };

    bpf_perf_event_output(ctx, &events, BPF_F_CURRENT_CPU, &event, sizeof(event));

    return 0;
}

SEC("kprobe/tcp_close")
int bpf_call_tcp_close(struct pt_regs *ctx) {
    struct sock *sk = (void *) PT_REGS_PARM1(ctx);

    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    u32 tid = pid_tgid;

    struct sock_common sc;
    bpf_probe_read(&sc, sizeof(sc), &sk->__sk_common);

    struct event event = {
        .event = 5,
        .pid   = pid,
        .proto = 6,
        .saddr = sc.skc_rcv_saddr,
        .sport = sc.skc_num,
        .daddr = sc.skc_daddr,
        .dport = ntohs(sc.skc_dport),
    };

    int rc = bpf_perf_event_output(ctx, &events, BPF_F_CURRENT_CPU, &event, sizeof(event));
    if (rc != 0) {
        bpf_printk("perf event output failure: %d\n", rc);
    }
    bpf_map_delete_elem(&socks, &tid);

    return 0;
}

char  _license[] SEC("license") = "GPL";
__u32 _version   SEC("version") = LINUX_VERSION_CODE;
