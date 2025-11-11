FROM cassandra:5

COPY Cassandra/entrypoint.sh /entrypoint.sh

EXPOSE 9042 7000

USER root

RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/bin/bash", "/entrypoint.sh"]
