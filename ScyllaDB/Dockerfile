FROM scylladb/scylla:2025.3

COPY entrypoint.sh /entrypoint.sh

EXPOSE 9042 7000 10000

ENTRYPOINT ["/bin/bash", "/entrypoint.sh"]
